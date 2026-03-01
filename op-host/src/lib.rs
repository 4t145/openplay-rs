pub mod auth;

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use axum::{
    http::StatusCode,
    middleware::Next,
    response::Response,
    routing::{get, post},
    Router,
};
use futures::future::BoxFuture;
use openplay_basic::user::{new_dyn_user_agent, UserId};
use openplay_host::{connection::ConnectionController, service::RoomServiceHandle};
use openplay_ua_http::{HttpUserAgent, HttpUserAgentConfig, HttpUserAgentState, Registry};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

use crate::auth::{ChallengeStore, JWT_TTL_SECS_DEFAULT, verify_jwt};

// ── AppState ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub registry: Registry,
    pub config: HttpUserAgentConfig,
    pub connection_controller: ConnectionController,
    /// JWT HMAC-SHA256 签名密钥
    pub jwt_secret: Arc<Vec<u8>>,
    /// JWT 有效期（秒）
    pub jwt_ttl_secs: u64,
    /// 挑战-响应认证的 challenge 缓存
    pub challenge_store: ChallengeStore,
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
            let agent = HttpUserAgent::new(user_id.clone(), registry, &config);
            let dyn_agent = new_dyn_user_agent(agent);
            controller
                .user_connect(user_id, dyn_agent)
                .await
                .map_err(|e| e.to_string())
        })
    }
}

// ── run_server ────────────────────────────────────────────────────────────────

pub async fn run_server(
    addr: SocketAddr,
    registry: Registry,
    connection_controller: ConnectionController,
    jwt_secret: Vec<u8>,
    jwt_ttl_secs: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let jwt_secret = Arc::new(jwt_secret);

    let app_state = AppState {
        registry: registry.clone(),
        config: HttpUserAgentConfig::default(),
        connection_controller,
        jwt_secret: jwt_secret.clone(),
        jwt_ttl_secs: jwt_ttl_secs.unwrap_or(JWT_TTL_SECS_DEFAULT),
        challenge_store: ChallengeStore::new(),
    };

    // /room 路由：
    //   /room/auth/challenge  —— 无需鉴权，获取 challenge
    //   /room/auth/verify     —— 无需鉴权，验证签名并颁发 JWT
    //   /room/ua              —— 需要 JWT 鉴权
    let room_router = Router::new()
        // 认证子路由（不经过 JWT 中间件）
        .route("/auth/challenge", post(auth::challenge_handler))
        .route("/auth/verify", post(auth::verify_handler))
        // 游戏 UA 子路由（经过 JWT 中间件）
        .route(
            "/ua",
            get(openplay_ua_http::sse_handler::<AppState>)
                .post(openplay_ua_http::action_handler::<AppState>)
                .delete(openplay_ua_http::disconnect_handler::<AppState>),
        )
        .with_state(app_state)
        .layer(axum::middleware::from_fn_with_state(
            jwt_secret,
            jwt_auth_middleware,
        ));
        
    let app = Router::new()
        .nest("/room", room_router)
        .route("/health", get(|| async { "ok" }))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// ── JWT 鉴权中间件 ────────────────────────────────────────────────────────────
//
// 注意：/room/auth/* 路由也会经过此中间件，但认证路由本身不需要 JWT。
// 这里对 /auth/ 前缀的路径直接放行，其余路径要求合法 JWT。

async fn jwt_auth_middleware(
    axum::extract::State(jwt_secret): axum::extract::State<Arc<Vec<u8>>>,
    req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // /room/auth/* 路径无需鉴权，直接放行
    if req.uri().path().contains("/auth/") {
        return Ok(next.run(req).await);
    }

    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = match auth_header.and_then(|h| h.strip_prefix("Bearer ")) {
        Some(t) => t.trim(),
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    match verify_jwt(token, &jwt_secret) {
        Some(user_id) => {
            let mut req = req;
            req.extensions_mut().insert(user_id);
            Ok(next.run(req).await)
        }
        None => {
            tracing::warn!("JWT 验证失败或已过期");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

// ── RoomServer ────────────────────────────────────────────────────────────────

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
