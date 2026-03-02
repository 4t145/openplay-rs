//! 用户身份模块
//!
//! 管理 ed25519 密钥对和用户名片（User），支持 JSON 文件持久化。
//!
//! # 本地存储格式
//! 每个身份以一个 JSON 文件保存，文件名为 `<user_id_base64url>.json`（去掉 padding）。
//! 文件内容见 [`IdentityFile`]。
//!
//! # 默认存储路径
//! 由 [`default_user_dir`] 返回，各平台规则：
//! - Linux：`$XDG_DATA_HOME/openplay/user/`（通常为 `~/.local/share/openplay/user/`）
//! - macOS：`~/Library/Application Support/openplay/user/`
//! - Windows：`%APPDATA%\openplay\user\`

use std::path::{Path, PathBuf};

use base64::prelude::*;
use ed25519_dalek::{SigningKey, VerifyingKey};
use openplay_basic::user::{User, UserId};
use rand::Rng;
use serde::{Deserialize, Serialize};

// ── 错误类型 ──────────────────────────────────────────────────────────────────

/// 身份操作错误
///
/// # ERROR
/// - [`IdentityError::Io`]：文件读写失败（权限不足、磁盘满等）
/// - [`IdentityError::Json`]：JSON 解析/序列化失败（文件损坏）
/// - [`IdentityError::InvalidKey`]：私钥 base64 解码或格式不正确
/// - [`IdentityError::NoDirFound`]：平台数据目录不存在且无法确定
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("私钥格式无效: {0}")]
    InvalidKey(String),

    #[error("无法确定平台数据目录")]
    NoDirFound,
}

// ── 持久化结构 ────────────────────────────────────────────────────────────────

/// 身份文件的 JSON 结构（持久化到磁盘的内容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityFile {
    /// 私钥，标准 base64 编码（32 字节 ed25519 seed）
    pub signing_key: String,
    /// 用户名片（公开展示信息）
    pub user: UserCard,
}

/// 用户名片（公开展示信息，不含私钥）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCard {
    /// 显示昵称
    pub nickname: String,
    /// 头像 URL（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// 是否为机器人（客户端身份文件通常为 false）
    #[serde(default)]
    pub is_bot: bool,
}

// ── KeyPair ───────────────────────────────────────────────────────────────────

/// 用户身份：ed25519 密钥对 + 用户名片
///
/// 公钥即 [`UserId`]，可直接作为身份标识。
#[derive(Clone)]
pub struct KeyPair {
    /// ed25519 私钥（含公钥）
    signing_key: SigningKey,
    /// 用户名片
    pub card: UserCard,
}

impl KeyPair {
    /// 生成一个新的随机密钥对。
    pub fn generate(nickname: impl Into<String>) -> Self {
        let mut seed = [0u8; 32];
        rand::rng().fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        Self {
            signing_key,
            card: UserCard {
                nickname: nickname.into(),
                avatar_url: None,
                is_bot: false,
            },
        }
    }

    /// 返回 ed25519 公钥对应的 [`UserId`]。
    pub fn user_id(&self) -> UserId {
        let vk: VerifyingKey = self.signing_key.verifying_key();
        UserId::from_bytes(*vk.as_bytes())
    }

    /// 将当前身份转换为框架的 [`User`] 结构（不含私钥）。
    pub fn to_user(&self) -> User {
        User {
            id: self.user_id(),
            nickname: self.card.nickname.clone(),
            avatar_url: self.card.avatar_url.clone(),
            is_bot: self.card.is_bot,
        }
    }

    /// 返回内部私钥的引用（用于签名）。
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    // ── 持久化 ────────────────────────────────────────────────────────────────

    /// 将身份保存到指定路径（覆盖已存在的文件）。
    ///
    /// # ERROR
    /// - 父目录不存在时返回 [`IdentityError::Io`]（不自动创建多级目录）
    /// - JSON 序列化失败返回 [`IdentityError::Json`]
    pub fn save(&self, path: &Path) -> Result<(), IdentityError> {
        let file = IdentityFile {
            signing_key: BASE64_STANDARD.encode(self.signing_key.as_bytes()),
            user: self.card.clone(),
        };
        let json = serde_json::to_string_pretty(&file)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// 从指定路径加载身份。
    ///
    /// # ERROR
    /// - 文件不存在或无读取权限返回 [`IdentityError::Io`]
    /// - JSON 解析失败返回 [`IdentityError::Json`]
    /// - 私钥 base64 解码失败或长度不符返回 [`IdentityError::InvalidKey`]
    pub fn load(path: &Path) -> Result<Self, IdentityError> {
        let json = std::fs::read_to_string(path)?;
        let file: IdentityFile = serde_json::from_str(&json)?;

        let key_bytes = BASE64_STANDARD
            .decode(&file.signing_key)
            .map_err(|e| IdentityError::InvalidKey(format!("base64 解码失败: {}", e)))?;

        let key_array: [u8; 32] = key_bytes
            .try_into()
            .map_err(|_| IdentityError::InvalidKey("私钥长度不正确，期望 32 字节".to_string()))?;

        let signing_key = SigningKey::from_bytes(&key_array);

        Ok(Self {
            signing_key,
            card: file.user,
        })
    }
}

// ── 路径工具 ──────────────────────────────────────────────────────────────────

/// 返回本平台的 OpenPlay 用户身份目录。
///
/// | 平台 | 路径 |
/// |------|------|
/// | Linux | `$XDG_DATA_HOME/openplay/user/`（通常 `~/.local/share/openplay/user/`）|
/// | macOS | `~/Library/Application Support/openplay/user/` |
/// | Windows | `%APPDATA%\openplay\user\` |
///
/// # ERROR
/// - 若平台无法确定数据目录，返回 [`IdentityError::NoDirFound`]
pub fn default_user_dir() -> Result<PathBuf, IdentityError> {
    let base = dirs::data_dir().ok_or(IdentityError::NoDirFound)?;
    Ok(base.join("openplay").join("user"))
}

/// 根据 `user_id` 计算身份文件名（`<base64url_no_pad>.json`）。
pub fn identity_filename(user_id: &UserId) -> String {
    // 用 URL-safe base64 去掉 padding 作为文件名，避免 `=` 和 `+` `/ 在路径中的问题
    let encoded = BASE64_URL_SAFE_NO_PAD.encode(user_id.as_bytes());
    format!("{}.json", encoded)
}

/// 列出目录下所有可用的身份文件，返回 `(path, KeyPair)` 列表。
///
/// 无法解析的文件会被跳过并打印 warn 日志。
///
/// # ERROR
/// - 目录不存在或无读取权限返回 [`IdentityError::Io`]
pub fn list_identities(dir: &Path) -> Result<Vec<(PathBuf, KeyPair)>, IdentityError> {
    let mut result = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        match KeyPair::load(&path) {
            Ok(kp) => result.push((path, kp)),
            Err(e) => tracing::warn!("跳过无法解析的身份文件 {:?}: {}", path, e),
        }
    }
    Ok(result)
}

// ── 便捷函数 ──────────────────────────────────────────────────────────────────

/// 确保目录存在（递归创建）。
pub fn ensure_user_dir(dir: &Path) -> Result<(), IdentityError> {
    std::fs::create_dir_all(dir)?;
    Ok(())
}

/// 在指定目录下加载第一个找到的身份；若目录为空则生成新身份并保存。
///
/// 这是最简单的"开箱即用"入口，适合 CLI / TUI 在无配置时调用。
///
/// # ERROR
/// - 目录创建失败返回 [`IdentityError::Io`]
/// - 身份文件读写失败返回 [`IdentityError::Io`] / [`IdentityError::Json`]
pub fn load_or_create(dir: &Path, default_nickname: &str) -> Result<KeyPair, IdentityError> {
    ensure_user_dir(dir)?;

    // 尝试加载已有身份
    let entries = list_identities(dir)?;
    if let Some((_path, kp)) = entries.into_iter().next() {
        tracing::info!("loading existing identity: {} ({})", kp.card.nickname, kp.user_id());
        return Ok(kp);
    }

    // 无身份，生成新的
    let kp = KeyPair::generate(default_nickname);
    let filename = identity_filename(&kp.user_id());
    let path = dir.join(&filename);
    kp.save(&path)?;
    tracing::info!("generated new identity: {} -> {:?}", kp.user_id(), path);
    Ok(kp)
}
