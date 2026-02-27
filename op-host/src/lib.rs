use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use axum::{
    extract::{FromRequestParts, State},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
    routing::get,
    Json, Router,
};
use openplay_basic::user::UserId;
use openplay_host::{connection::ConnectionController, service::RoomServiceHandle};
use openplay_ua_http::{HttpUserAgentConfig, HttpUserAgentState, Registry, UserAgentChannels};
use serde::{Deserialize, Serialize};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};

#[derive(Clone)]
pub struct AppState {
    pub registry: Registry,
    pub config: HttpUserAgentConfig,
    pub connection_controller: ConnectionController,
}

#[async_trait::async_trait]
impl HttpUserAgentState for AppState {
    fn registry(&self) -> &Registry {
        &self.registry
    }

    fn config(&self) -> &HttpUserAgentConfig {
        &self.config
    }

    async fn disconnect(&self, user_id: &UserId) -> Result<(), String> {
        info!("Requesting disconnect for user {}", user_id);
        match self.connection_controller.user_disconnect(user_id.clone()).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
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
        .route("/events", get(openplay_ua_http::sse_handler::<AppState>))
        .route("/action", axum::routing::post(openplay_ua_http::action_handler::<AppState>))
        .route("/disconnect", axum::routing::delete(openplay_ua_http::disconnect_handler::<AppState>))
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
        Some(auth_header) if auth_header.starts_with("Bearer ") => {
            let token = auth_header[7..].trim();
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
