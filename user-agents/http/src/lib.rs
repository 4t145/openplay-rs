use std::{
    collections::HashMap,
    convert::Infallible,
    sync::{Arc, RwLock},
    time::Duration,
};

use axum::{
    extract::{Extension, State},
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use futures::{future::BoxFuture, stream::Stream, StreamExt};
use openplay_basic::{
    room::Update,
    user::{ActionData, UserAgent, UserId},
};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_stream::wrappers::BroadcastStream;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{debug, error, info, warn};

/// Configuration for the HTTP agent.
#[derive(Debug, Clone)]
pub struct HttpUserAgentConfig {
    /// Keep-alive interval for SSE connections.
    pub keep_alive_interval: Duration,
    /// Capacity of the broadcast channel for updates.
    pub broadcast_capacity: usize,
    /// Capacity of the MPSC channel for actions.
    pub action_capacity: usize,
}

impl Default for HttpUserAgentConfig {
    fn default() -> Self {
        Self {
            keep_alive_interval: Duration::from_secs(15),
            broadcast_capacity: 100,
            action_capacity: 100,
        }
    }
}

/// Trait defining the state requirements for the HTTP agent handlers.
/// Implement this on your application state to integrate the agent handlers.
pub trait HttpUserAgentState: Clone + Send + Sync + 'static {
    fn registry(&self) -> &Registry;
    fn config(&self) -> &HttpUserAgentConfig;
    /// Optional: Handle explicit disconnection request from the HTTP layer.
    fn disconnect(&self, _user_id: &UserId) -> BoxFuture<'static, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }
    /// Optional: Auto-connect a user when they attempt SSE but are not yet in the registry.
    /// The implementation should create an HttpUserAgent, register it, and connect to the game.
    fn connect(&self, _user_id: &UserId) -> BoxFuture<'static, Result<(), String>> {
        Box::pin(async { Err("Auto-connect not supported".to_string()) })
    }
}

/// Channels for communication between the HTTP layer and the game logic.
#[derive(Clone)]
pub struct UserAgentChannels {
    /// Channel to send game updates to the connected HTTP client (via SSE).
    pub tx_update: broadcast::Sender<Arc<Update>>,
    /// Channel to receive player actions from the HTTP client (via POST).
    pub tx_action: mpsc::Sender<ActionData>,
}

/// A registry to store active agent channels, keyed by UserId.
/// This allows the HTTP handlers to find the correct channels for a given user.
pub type Registry = Arc<RwLock<HashMap<UserId, UserAgentChannels>>>;

/// The UserAgent implementation that bridges the game logic with HTTP.
pub struct HttpUserAgent {
    user_id: UserId,
    registry: Registry,
    tx_update: broadcast::Sender<Arc<Update>>,
    rx_action: Mutex<mpsc::Receiver<ActionData>>,
}

impl HttpUserAgent {
    /// Creates a new HttpUserAgent and registers it in the provided registry.
    ///
    /// Returns the agent instance which should be passed to the game logic.
    pub fn new(user_id: UserId, registry: Registry, config: &HttpUserAgentConfig) -> Self {
        // Create channels with configured capacities
        let (tx_update, _) = broadcast::channel(config.broadcast_capacity);
        let (tx_action, rx_action) = mpsc::channel(config.action_capacity);

        let channels = UserAgentChannels {
            tx_update: tx_update.clone(),
            tx_action: tx_action.clone(),
        };

        // Register channels
        {
            let mut reg = registry.write().unwrap();
            reg.insert(user_id.clone(), channels);
        }

        Self {
            user_id,
            registry,
            tx_update,
            rx_action: Mutex::new(rx_action),
        }
    }
}

impl Drop for HttpUserAgent {
    fn drop(&mut self) {
        // Cleanup when the agent is dropped (e.g., game ends or player disconnects from game logic)
        let mut reg = self.registry.write().unwrap();
        if reg.remove(&self.user_id).is_some() {
            debug!("Removed agent channels for user {}", self.user_id);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HttpUserAgentError {
    #[error("Failed to broadcast update")]
    BroadcastError,
}

impl UserAgent for HttpUserAgent {
    type Error = HttpUserAgentError;

    async fn send_update(&self, update: Update) -> Result<(), Self::Error> {
        // Broadcast the update to all connected SSE clients for this user
        // We wrap in Arc to avoid cloning the potentially large Update struct for every listener (though usually 1)
        // Ignoring send errors (no active listeners is fine)
        let _ = self.tx_update.send(Arc::new(update));
        Ok(())
    }

    async fn receive_action(&self) -> Result<Option<ActionData>, Self::Error> {
        // Wait for an action from the HTTP POST handler
        let mut rx = self.rx_action.lock().await;
        Ok(rx.recv().await)
    }

    async fn close(&self) {
        // Close channels? Usually dropping the agent is enough.
        // We can explicitly remove from registry here if we want to support explicit close.
        let mut reg = self.registry.write().unwrap();
        reg.remove(&self.user_id);
    }
}

// --- Axum Handlers ---

/// Default implementation of HttpUserAgentState for simple use cases.
#[derive(Clone)]
pub struct DefaultUserAgentState {
    pub registry: Registry,
    pub config: HttpUserAgentConfig,
}

impl HttpUserAgentState for DefaultUserAgentState {
    fn registry(&self) -> &Registry {
        &self.registry
    }
    fn config(&self) -> &HttpUserAgentConfig {
        &self.config
    }
}

/// Handler for Server-Sent Events (SSE).
/// Expects the UserId to be present in the request extensions (injected by auth middleware).
/// If the user is not in the registry, attempts auto-connect via `state.connect()`.
pub async fn sse_handler<S>(
    State(state): State<S>,
    Extension(user_id): Extension<UserId>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>>
where
    S: HttpUserAgentState,
{
    debug!("SSE connection request for user {}", user_id);
    let config = state.config();

    // Try to find the user in registry; if not found, attempt auto-connect
    let mut stream_rx = {
        let reg = state.registry().read().unwrap();
        reg.get(&user_id).map(|ch| ch.tx_update.subscribe())
    };

    if stream_rx.is_none() {
        debug!("User {} not in registry, attempting auto-connect", user_id);
        match state.connect(&user_id).await {
            Ok(()) => {
                info!("Auto-connected user {}", user_id);
                let reg = state.registry().read().unwrap();
                stream_rx = reg.get(&user_id).map(|ch| ch.tx_update.subscribe());
            }
            Err(e) => {
                warn!("Auto-connect failed for user {}: {}", user_id, e);
            }
        }
    }

    let stream = async_stream::stream! {
        if let Some(rx) = stream_rx {
             let mut broadcast_stream = BroadcastStream::new(rx);
             while let Some(res) = broadcast_stream.next().await {
                 match res {
                     Ok(update) => {
                         match serde_json::to_string(&*update) {
                             Ok(json) => yield Ok(Event::default().data(json)),
                             Err(e) => {
                                 error!("Failed to serialize update: {}", e);
                                 yield Ok(Event::default().event("error").data("serialization error"));
                             }
                         }
                     }
                     Err(_missed) => {
                         yield Ok(Event::default().event("error").data("missed messages"));
                     }
                 }
             }
        } else {
             warn!("User {} not found in registry for SSE (after auto-connect attempt)", user_id);
             yield Ok(Event::default().event("error").data("user not registered"));
        }
    };

    Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new().interval(config.keep_alive_interval))
}

/// Handler for receiving user actions via HTTP POST.
/// Expects the UserId to be present in the request extensions.
pub async fn action_handler<S>(
    State(state): State<S>,
    Extension(user_id): Extension<UserId>,
    Json(action_data): Json<ActionData>,
) -> impl IntoResponse
where
    S: HttpUserAgentState,
{
    debug!("Received action for user {}: {:?}", user_id, action_data);

    let tx_action = {
        let reg = state.registry().read().unwrap();
        if let Some(channels) = reg.get(&user_id) {
            Some(channels.tx_action.clone())
        } else {
            None
        }
    };

    if let Some(tx) = tx_action {
        match tx.send(action_data).await {
            Ok(_) => (axum::http::StatusCode::OK, "Action accepted").into_response(),
            Err(_) => (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "Game ended or agent closed",
            )
                .into_response(),
        }
    } else {
        (axum::http::StatusCode::NOT_FOUND, "User agent not active").into_response()
    }
}

/// Handler for explicit disconnect requests via HTTP DELETE.
/// Expects the UserId to be present in the request extensions.
pub async fn disconnect_handler<S>(
    State(state): State<S>,
    Extension(user_id): Extension<UserId>,
) -> impl IntoResponse
where
    S: HttpUserAgentState,
{
    debug!("Explicit disconnect request for user {}", user_id);

    // Call the application-specific disconnect logic
    match state.disconnect(&user_id).await {
        Ok(_) => {
            // Also cleanup channels if present
            let mut reg = state.registry().write().unwrap();
            if reg.remove(&user_id).is_some() {
                debug!(
                    "Removed agent channels for user {} on explicit disconnect",
                    user_id
                );
            }
            (axum::http::StatusCode::OK, "Disconnected").into_response()
        }
        Err(e) => {
            warn!("Failed to disconnect user {}: {}", user_id, e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to disconnect: {}", e),
            )
                .into_response()
        }
    }
}

/// Helper to create a Router with the standard routes using DefaultUserAgentState.
/// For advanced integration, construct your own Router with your custom State implementing HttpUserAgentState.
///
/// Note: You MUST add a middleware that extracts authentication and inserts `UserId` into extensions
/// before mounting this router or calling these handlers.
pub fn router(registry: Registry) -> Router {
    let state = DefaultUserAgentState {
        registry,
        config: HttpUserAgentConfig::default(),
    };

    Router::new()
        .route("/events", get(sse_handler::<DefaultUserAgentState>))
        .route("/action", post(action_handler::<DefaultUserAgentState>))
        .route(
            "/disconnect",
            axum::routing::delete(disconnect_handler::<DefaultUserAgentState>),
        )
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
