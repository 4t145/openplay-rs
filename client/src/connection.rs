//! 房间连接模块
//!
//! [`RoomClient`] 封装了与 op-host 房间的全部 HTTP 通信：
//! - SSE 长连接：接收服务端推送的 [`Update`]
//! - 发送动作：`POST /room/ua`
//! - 主动断开：`DELETE /room/ua`
//!
//! 创建 `RoomClient` 前需先通过 [`crate::auth::authenticate`] 获取 JWT token。

use futures::stream::{Stream, StreamExt};
use openplay_basic::{room::Update, user::ActionData};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest_eventsource::EventSource;

// ── 错误类型 ──────────────────────────────────────────────────────────────────

/// 连接/通信错误
///
/// # ERROR
/// - [`ConnectionError::Http`]：底层 HTTP 错误（网络、超时等）
/// - [`ConnectionError::InvalidToken`]：JWT token 包含非 ASCII 字符，无法写入 header
/// - [`ConnectionError::ServerError`]：服务端返回非 2xx 状态码
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("HTTP 错误: {0}")]
    Http(#[from] reqwest::Error),

    #[error("无效的 token（非 ASCII）: {0}")]
    InvalidToken(String),

    #[error("服务端错误 {status}: {body}")]
    ServerError { status: u16, body: String },
}

// ── SSE 事件 ──────────────────────────────────────────────────────────────────

/// SSE 流事件
pub enum SseEvent {
    /// SSE 连接已成功建立
    Connected,
    /// 服务端推送了一次房间/游戏更新
    Update(Update),
}

// ── RoomClient ────────────────────────────────────────────────────────────────

/// 与 op-host 单个房间通信的客户端。
///
/// # 使用流程
/// ```text
/// let token = openplay_client::auth::authenticate(&base_url, &key_pair).await?;
/// let client = RoomClient::new(base_url, "/room/ua".into(), token)?;
/// let stream = client.connect_sse();   // 订阅 SSE
/// client.send_action(action).await?;   // 发送动作
/// client.disconnect().await?;          // 断开
/// ```
#[derive(Clone)]
pub struct RoomClient {
    http: reqwest::Client,
    base_url: String,
    room_path: String,
    /// 用户公钥的字符串表示（base64），供调用方查询
    pub user_id: String,
}

impl RoomClient {
    /// 创建新的 RoomClient。
    ///
    /// `token` 为通过 [`crate::auth::authenticate`] 获取的 JWT。
    /// `user_id` 为 base64 编码的公钥字符串（仅供记录/展示用）。
    ///
    /// # ERROR
    /// - token 含非 ASCII 字符 → [`ConnectionError::InvalidToken`]
    /// - HTTP client 构建失败 → [`ConnectionError::Http`]
    pub fn new(
        base_url: String,
        room_path: String,
        token: String,
        user_id: String,
    ) -> Result<Self, ConnectionError> {
        let mut headers = HeaderMap::new();
        let auth_value = format!("Bearer {}", token);
        let header_val = HeaderValue::from_str(&auth_value)
            .map_err(|_| ConnectionError::InvalidToken(token.clone()))?;
        headers.insert(AUTHORIZATION, header_val);

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(ConnectionError::Http)?;

        Ok(Self {
            http,
            base_url,
            room_path,
            user_id,
        })
    }

    /// 房间 UA 端点的完整 URL。
    fn url(&self) -> String {
        format!(
            "{}{}",
            self.base_url.trim_end_matches('/'),
            self.room_path
        )
    }

    /// 连接 SSE 流，返回异步流。
    ///
    /// 首先 yield [`SseEvent::Connected`] 表示连接已建立，
    /// 随后每次服务端推送均 yield [`SseEvent::Update`]。
    /// 连接关闭或出错时流结束。
    pub fn connect_sse(&self) -> impl Stream<Item = Result<SseEvent, anyhow::Error>> + Send + 'static {
        let url = self.url();
        let http = self.http.clone();

        async_stream::stream! {
            let request = http.get(&url);
            let mut es = match EventSource::new(request) {
                Ok(es) => es,
                Err(e) => {
                    yield Err(anyhow::anyhow!("创建 EventSource 失败: {}", e));
                    return;
                }
            };

            while let Some(event) = es.next().await {
                match event {
                    Ok(reqwest_eventsource::Event::Open) => {
                        tracing::info!("SSE Connection opened");
                        yield Ok(SseEvent::Connected);
                    }
                    Ok(reqwest_eventsource::Event::Message(msg)) => {
                        match serde_json::from_str::<Update>(&msg.data) {
                            Ok(update) => yield Ok(SseEvent::Update(update)),
                            Err(e) => {
                                tracing::warn!(
                                    "SSE 消息解析失败: {}, 原始数据: {}",
                                    e,
                                    msg.data
                                );
                            }
                        }
                    }
                    Err(reqwest_eventsource::Error::StreamEnded) => {
                        tracing::info!("SSE 流已结束");
                        break;
                    }
                    Err(e) => {
                        yield Err(anyhow::anyhow!("SSE 错误: {}", e));
                        break;
                    }
                }
            }
            es.close();
        }
    }

    /// 发送游戏/房间动作（`POST /room/ua`）。
    ///
    /// # ERROR
    /// - 网络错误 → [`ConnectionError::Http`]
    /// - 服务端拒绝 → [`ConnectionError::ServerError`]
    pub async fn send_action(&self, action: ActionData) -> Result<(), ConnectionError> {
        let url = self.url();
        let resp = self
            .http
            .post(&url)
            .json(&action)
            .send()
            .await
            .map_err(ConnectionError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ConnectionError::ServerError { status, body });
        }
        Ok(())
    }

    /// 主动断开连接（`DELETE /room/ua`）。
    ///
    /// 服务端即使返回错误也只记录 warn 日志，不返回 Error，
    /// 因为断开时服务端可能已经关闭了该会话。
    pub async fn disconnect(&self) -> Result<(), ConnectionError> {
        let url = self.url();
        let resp = self
            .http
            .delete(&url)
            .send()
            .await
            .map_err(ConnectionError::Http)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!("断开连接响应: {} - {}", status, body);
        }
        Ok(())
    }
}
