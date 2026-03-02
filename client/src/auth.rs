//! 挑战-响应认证模块
//!
//! 封装与 op-host 的完整认证流程：
//! 1. `POST /room/auth/challenge` — 提交公钥，获取随机 challenge
//! 2. 用 ed25519 私钥对 challenge 签名
//! 3. `POST /room/auth/verify` — 提交签名，服务端验证后颁发 JWT
//!
//! 调用方只需一个 [`authenticate`] 函数即可完成全流程。

use base64::prelude::*;
use ed25519_dalek::Signer;
use serde::{Deserialize, Serialize};

use crate::identity::KeyPair;

// ── 错误类型 ──────────────────────────────────────────────────────────────────

/// 认证过程错误
///
/// # ERROR
/// - [`AuthError::Http`]：HTTP 请求失败（网络不通、服务端宕机等）
/// - [`AuthError::ServerError`]：服务端返回非 2xx 状态码
/// - [`AuthError::InvalidResponse`]：响应 JSON 解析失败（协议不匹配）
/// - [`AuthError::InvalidChallenge`]：challenge base64 解码失败或长度不符
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),

    #[error("服务端错误 {status}: {body}")]
    ServerError { status: u16, body: String },

    #[error("响应解析失败: {0}")]
    InvalidResponse(String),

    #[error("challenge 格式无效: {0}")]
    InvalidChallenge(String),
}

// ── 请求/响应类型（与 op-host/src/auth.rs 保持一致）────────────────────────────

#[derive(Serialize)]
struct ChallengeRequest {
    user_id: String,
}

#[derive(Deserialize)]
struct ChallengeResponse {
    challenge: String,
}

#[derive(Serialize)]
struct VerifyRequest {
    user_id: String,
    signature: String,
}

#[derive(Deserialize)]
struct VerifyResponse {
    token: String,
}

// ── 公开 API ──────────────────────────────────────────────────────────────────

/// 完成完整的 challenge-response 认证流程，返回 JWT token。
///
/// `base_url` 示例：`"http://localhost:3000"`
///
/// 内部步骤：
/// 1. `POST {base_url}/room/auth/challenge` 获取 challenge
/// 2. 用 `key_pair` 的私钥对 challenge 签名
/// 3. `POST {base_url}/room/auth/verify` 提交签名，获取 JWT
///
/// # ERROR
/// - 网络错误 → [`AuthError::Http`]
/// - 服务端拒绝（错误状态码）→ [`AuthError::ServerError`]
/// - 响应 JSON 格式错误 → [`AuthError::InvalidResponse`]
/// - challenge base64 格式错误 → [`AuthError::InvalidChallenge`]
pub async fn authenticate(base_url: &str, key_pair: &KeyPair) -> Result<String, AuthError> {
    let client = reqwest::Client::new();
    let user_id = key_pair.user_id().to_string();

    // ── 步骤 1：获取 challenge ─────────────────────────────────────────────────
    let challenge_url = format!("{}/room/auth/challenge", base_url.trim_end_matches('/'));
    let resp = client
        .post(&challenge_url)
        .json(&ChallengeRequest {
            user_id: user_id.clone(),
        })
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(AuthError::ServerError { status, body });
    }

    let challenge_resp: ChallengeResponse = resp.json().await.map_err(|e| {
        AuthError::InvalidResponse(format!("challenge 响应解析失败: {}", e))
    })?;

    // ── 步骤 2：签名 ──────────────────────────────────────────────────────────
    let challenge_bytes = BASE64_STANDARD
        .decode(&challenge_resp.challenge)
        .map_err(|e| AuthError::InvalidChallenge(format!("base64 解码失败: {}", e)))?;

    let signature = key_pair.signing_key().sign(&challenge_bytes);
    let signature_b64 = BASE64_STANDARD.encode(signature.to_bytes());

    // ── 步骤 3：提交签名，获取 JWT ────────────────────────────────────────────
    let verify_url = format!("{}/room/auth/verify", base_url.trim_end_matches('/'));
    let resp = client
        .post(&verify_url)
        .json(&VerifyRequest {
            user_id: user_id.clone(),
            signature: signature_b64,
        })
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(AuthError::ServerError { status, body });
    }

    let verify_resp: VerifyResponse = resp.json().await.map_err(|e| {
        AuthError::InvalidResponse(format!("verify 响应解析失败: {}", e))
    })?;

    tracing::info!("verification succeeded={}", user_id);
    Ok(verify_resp.token)
}
