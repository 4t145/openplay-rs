//! 挑战-响应认证模块
//!
//! 流程：
//!   1. POST /room/auth/challenge  —— 客户端提交公钥，服务端返回随机 challenge
//!   2. POST /room/auth/verify     —— 客户端提交公钥 + 签名，服务端验证后颁发 JWT
//!
//! 此后所有需要鉴权的请求携带 `Authorization: Bearer <JWT>`。

use std::{sync::Arc, time::Duration};

use axum::{Json, extract::State, http::StatusCode};
use base64::prelude::*;
use dashmap::DashMap;
use ed25519_dalek::{Signature, VerifyingKey};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use openplay_basic::user::UserId;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use crate::AppState;

// ── 常量 ──────────────────────────────────────────────────────────────────────

/// Challenge 有效期（秒）
const CHALLENGE_TTL_SECS: u64 = 30;
/// Challenge 字节长度
const CHALLENGE_BYTES: usize = 32;
/// JWT 默认有效期（秒）
pub const JWT_TTL_SECS_DEFAULT: u64 = 86_400; // 24 小时

// ── ChallengeStore ────────────────────────────────────────────────────────────

struct ChallengeEntry {
    challenge: [u8; CHALLENGE_BYTES],
    issued_at: Instant,
}

/// 线程安全的一次性 challenge 缓存。
/// Key 为 UserId（公钥），value 为 challenge 字节 + 签发时间。
#[derive(Clone, Default)]
pub struct ChallengeStore(Arc<DashMap<UserId, ChallengeEntry>>);

impl ChallengeStore {
    pub fn new() -> Self {
        Self(Arc::new(DashMap::new()))
    }

    /// 为指定用户生成并缓存一个新 challenge，覆盖之前未使用的 challenge。
    pub fn insert(&self, user_id: UserId) -> [u8; CHALLENGE_BYTES] {
        let mut challenge = [0u8; CHALLENGE_BYTES];
        rand::rng().fill_bytes(&mut challenge);
        self.0.insert(
            user_id,
            ChallengeEntry {
                challenge,
                issued_at: Instant::now(),
            },
        );
        challenge
    }

    /// 取出并消费 challenge（一次性）。若不存在或已过期则返回 None。
    pub fn take(&self, user_id: &UserId) -> Option<[u8; CHALLENGE_BYTES]> {
        let entry = self.0.remove(user_id)?;
        if entry.1.issued_at.elapsed() > Duration::from_secs(CHALLENGE_TTL_SECS) {
            return None; // 已过期，丢弃
        }
        Some(entry.1.challenge)
    }
}

// ── JWT ───────────────────────────────────────────────────────────────────────

/// JWT payload
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject：base64 编码的 UserId（公钥）
    pub sub: String,
    /// 过期时间（Unix 时间戳，秒）
    pub exp: u64,
}

/// 颁发 JWT
pub fn issue_jwt(
    user_id: &UserId,
    secret: &[u8],
    ttl_secs: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let claims = Claims {
        sub: user_id.to_string(),
        exp: now + ttl_secs,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret))
}

/// 验证 JWT 并提取 UserId
pub fn verify_jwt(token: &str, secret: &[u8]) -> Option<UserId> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )
    .ok()?;
    UserId::try_from(token_data.claims.sub.as_str()).ok()
}

// ── 请求 / 响应类型 ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ChallengeRequest {
    /// 客户端公钥，base64 编码（即 UserId 的字符串表示）
    pub user_id: String,
}

#[derive(Serialize)]
pub struct ChallengeResponse {
    /// 随机 challenge，base64 编码
    pub challenge: String,
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    /// 客户端公钥，base64 编码
    pub user_id: String,
    /// 私钥对 challenge 的 ed25519 签名，base64 编码
    pub signature: String,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub token: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /room/auth/challenge
///
/// 客户端提交公钥，服务端返回随机 challenge（base64）。
pub async fn challenge_handler(
    State(state): State<AppState>,
    Json(body): Json<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, StatusCode> {
    let user_id = UserId::try_from(body.user_id.as_str()).map_err(|e| {
        tracing::warn!("challenge_handler: 无效的 user_id: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    let challenge = state.challenge_store.insert(user_id);
    let encoded = BASE64_STANDARD.encode(challenge);

    Ok(Json(ChallengeResponse { challenge: encoded }))
}

/// POST /room/auth/verify
///
/// 客户端提交公钥 + 对 challenge 的签名，验证通过后颁发 JWT。
pub async fn verify_handler(
    State(state): State<AppState>,
    Json(body): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, StatusCode> {
    // 1. 解析公钥 → UserId
    let user_id = UserId::try_from(body.user_id.as_str()).map_err(|e| {
        tracing::warn!("verify_handler: 无效的 user_id: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    // 2. 取出并消费 challenge（一次性，防重放）
    let challenge = state.challenge_store.take(&user_id).ok_or_else(|| {
        tracing::warn!("verify_handler: challenge 不存在或已过期，user_id={}", user_id);
        StatusCode::UNAUTHORIZED
    })?;

    // 3. 解码签名（base64）
    let sig_bytes = BASE64_STANDARD.decode(&body.signature).map_err(|e| {
        tracing::warn!("verify_handler: 签名 base64 解码失败: {}", e);
        StatusCode::BAD_REQUEST
    })?;
    let signature = Signature::from_slice(&sig_bytes).map_err(|e| {
        tracing::warn!("verify_handler: 签名格式不正确: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    // 4. 构建 ed25519 验证密钥（UserId 即公钥字节）
    let verifying_key = VerifyingKey::from_bytes(user_id.as_bytes()).map_err(|e| {
        tracing::warn!("verify_handler: 公钥格式不正确: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    // 5. 验证签名
    use ed25519_dalek::Verifier;
    verifying_key.verify(&challenge, &signature).map_err(|_| {
        tracing::warn!("verify_handler: 签名验证失败，user_id={}", user_id);
        StatusCode::UNAUTHORIZED
    })?;

    // 6. 颁发 JWT
    let token = issue_jwt(&user_id, &state.jwt_secret, state.jwt_ttl_secs).map_err(|e| {
        tracing::error!("verify_handler: JWT 签发失败: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("用户认证成功: {}", user_id);
    Ok(Json(VerifyResponse { token }))
}
