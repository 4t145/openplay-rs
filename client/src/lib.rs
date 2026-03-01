//! OpenPlay 客户端库
//!
//! 提供与 op-host 服务器交互的完整客户端能力：
//!
//! | 模块 | 功能 |
//! |------|------|
//! | [`identity`] | ed25519 密钥对管理、用户名片、本地 JSON 持久化 |
//! | [`auth`] | challenge-response 认证流程，获取 JWT token |
//! | [`connection`] | [`RoomClient`]：SSE 订阅、发送动作、断开连接 |
//!
//! # 典型使用流程
//!
//! ```rust,no_run
//! use openplay_client::{
//!     identity::{default_user_dir, load_or_create},
//!     auth::authenticate,
//!     connection::RoomClient,
//! };
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // 1. 加载或创建本地身份
//!     let user_dir = default_user_dir()?;
//!     let key_pair = load_or_create(&user_dir, "Player")?;
//!
//!     // 2. 与服务器完成认证，拿到 JWT
//!     let token = authenticate("http://localhost:3000", &key_pair).await?;
//!
//!     // 3. 建立房间连接
//!     let client = RoomClient::new(
//!         "http://localhost:3000".into(),
//!         "/room/ua".into(),
//!         token,
//!         key_pair.user_id().to_string(),
//!     )?;
//!
//!     // 4. 订阅 SSE 更新 / 发送动作
//!     let _stream = client.connect_sse();
//!     Ok(())
//! }
//! ```

pub mod auth;
pub mod connection;
pub mod identity;

// 便捷 re-export
pub use auth::authenticate;
pub use connection::{RoomClient, SseEvent};
pub use identity::{KeyPair, default_user_dir, load_or_create};
