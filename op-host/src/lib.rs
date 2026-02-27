use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use axum::{http::StatusCode, middleware::Next, response::Response, routing::get, Router};
use futures::future::BoxFuture;
use openplay_basic::user::{new_dyn_user_agent, UserId};
use openplay_host::{connection::ConnectionController, service::RoomServiceHandle};
use openplay_ua_http::{
    HttpUserAgent, HttpUserAgentConfig, HttpUserAgentState, Registry,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub registry: Registry,
    pub config: HttpUserAgentConfig,
    pub connection_controller: ConnectionController,
}

impl HttpUserAgentState for AppState {
    fn registry(&self) -> &Registry {
        &self.registry
    }

    fn config(&self) -> &HttpUserAgentConfig {
        &self.config
    }
    fn disconnect(&self, user_id: &UserId) -> BoxFuture<'static, Result<(), String>> {
        let controller = self.connection_controller.clone();
        let user_id = user_id.clone();
        Box::pin(async move {
            info!("Requesting disconnect for user {}", user_id);
            match controller.user_disconnect(user_id).await {
                Ok(_) => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        })
    }
    fn connect(&self, user_id: &UserId) -> BoxFuture<'static, Result<(), String>> {
        let registry = self.registry.clone();
        let config = self.config.clone();
        let controller = self.connection_controller.clone();
        let user_id = user_id.clone();
        Box::pin(async move {
            info!("Auto-connecting user {}", user_id);
            // Create HttpUserAgent (registers channels in Registry automatically)
            let agent = HttpUserAgent::new(user_id.clone(), registry, &config);
            let dyn_agent = new_dyn_user_agent(agent);
            // Register with the ConnectionController so the room's event loop picks it up
            controller
                .user_connect(user_id, dyn_agent)
                .await
                .map_err(|e| e.to_string())
        })
    }
}

pub async fn run_server(
    addr: SocketAddr,
    registry: Registry,
    connection_controller: ConnectionController,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState {
        registry: registry.clone(),
        config: HttpUserAgentConfig::default(),
        connection_controller,
    };

    let api_router = Router::new()
        .route(
            "/ua",
            get(openplay_ua_http::sse_handler::<AppState>)
                .post(openplay_ua_http::action_handler::<AppState>)
                .delete(openplay_ua_http::disconnect_handler::<AppState>),
        )
        .with_state(state.clone())
        // Add middleware to inject UserId from Authorization header
        .layer(axum::middleware::from_fn(auth_middleware));

    let app = Router::new()
        .nest("/room", api_router)
        .route("/health", get(|| async { "ok" }))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn auth_middleware(
    mut req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    match auth_header {
        Some(auth_header) => {
            let Some(token) = auth_header.strip_prefix("Bearer ") else {
                return Err(StatusCode::UNAUTHORIZED);
            };
            let token = token.trim();
            // In a real app, validate token. Here, we use token as UserId directly.
            if !token.is_empty() {
                use bytes::Bytes;
                let user_id = UserId::from(Bytes::from(token.to_string()));
                req.extensions_mut().insert(user_id);
                Ok(next.run(req).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

pub struct RoomServer {
    pub handle: RoomServiceHandle,
    pub registry: Registry,
}

impl RoomServer {
    pub async fn new(handle: RoomServiceHandle) -> Self {
        Self {
            handle,
            registry: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
