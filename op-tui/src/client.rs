use anyhow::{Context, Result};
use futures::stream::{Stream, StreamExt};
use openplay_basic::{
    room::Update,
    user::ActionData,
};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest_eventsource::EventSource;

/// Events from the SSE stream, distinguishing connection success from game updates.
pub enum SseEvent {
    /// SSE connection successfully opened.
    Connected,
    /// Server sent a game/room update.
    Update(Update),
}

/// HTTP + SSE client for communicating with the op-host server.
#[derive(Clone)]
pub struct GameClient {
    http: reqwest::Client,
    base_url: String,
    room_path: String,
    #[allow(dead_code)]
    user_id: String,
}

impl GameClient {
    pub fn new(base_url: String, room_path: String, user_id: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        let auth_value = format!("Bearer {}", user_id);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value).context("Invalid user_id for auth header")?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            http,
            base_url,
            room_path,
            user_id,
        })
    }

    #[allow(dead_code)]
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// The full URL for the room UA endpoint.
    fn url(&self) -> String {
        format!("{}{}", self.base_url, self.room_path)
    }

    /// Connect to SSE stream. Returns an owned stream of `SseEvent` values.
    /// Yields `SseEvent::Connected` when the connection opens, then `SseEvent::Update` for each
    /// server message. Errors are yielded as `Err(...)`.
    pub fn connect_sse(&self) -> impl Stream<Item = Result<SseEvent>> + Send + 'static {
        let url = self.url();
        let http = self.http.clone();

        async_stream::stream! {
            let request = http.get(&url);
            let mut es = EventSource::new(request).expect("Failed to create EventSource");

            while let Some(event) = es.next().await {
                match event {
                    Ok(reqwest_eventsource::Event::Open) => {
                        tracing::info!("SSE connection opened");
                        yield Ok(SseEvent::Connected);
                    }
                    Ok(reqwest_eventsource::Event::Message(msg)) => {
                        match serde_json::from_str::<Update>(&msg.data) {
                            Ok(update) => yield Ok(SseEvent::Update(update)),
                            Err(e) => {
                                tracing::warn!("Failed to parse SSE message: {}, raw data: {}", e, msg.data);
                            }
                        }
                    }
                    Err(reqwest_eventsource::Error::StreamEnded) => {
                        tracing::info!("SSE stream ended");
                        break;
                    }
                    Err(e) => {
                        yield Err(anyhow::anyhow!("SSE error: {}", e));
                        break;
                    }
                }
            }
            es.close();
        }
    }

    /// Send an action to the server via HTTP POST.
    pub async fn send_action(&self, action: ActionData) -> Result<()> {
        let url = self.url();
        let resp = self
            .http
            .post(&url)
            .json(&action)
            .send()
            .await
            .context("Failed to send action")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Action rejected: {} - {}", status, body);
        }
        Ok(())
    }

    /// Disconnect from the server via HTTP DELETE.
    pub async fn disconnect(&self) -> Result<()> {
        let url = self.url();
        let resp = self
            .http
            .delete(&url)
            .send()
            .await
            .context("Failed to send disconnect")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!("Disconnect response: {} - {}", status, body);
        }
        Ok(())
    }
}
