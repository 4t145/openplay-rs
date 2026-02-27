use bytes::Bytes;
use clap::Parser;
use config::{Config, Environment, File};
use op_host::{run_server, RoomServer};
use openplay_basic::room::{Room, RoomInfo};
use openplay_basic::user::{User, UserId};
use openplay_doudizhu::DouDizhuGame;
use openplay_host::service::RoomService;
use serde::Deserialize;
use std::net::{IpAddr, SocketAddr};
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// Port to listen on
    #[arg(short, long)]
    port: Option<u16>,

    /// Address to bind to
    #[arg(short('H'), long)]
    host: Option<String>,

    /// Game type
    #[arg(short, long, alias("game"))]
    app: Option<String>,

    /// Room ID
    #[arg(long)]
    room_id: Option<String>,

    /// Room Title
    #[arg(long)]
    title: Option<String>,

    /// Room Description
    #[arg(long)]
    description: Option<String>,

    /// Room Owner ID
    #[arg(long)]
    owner_id: Option<String>,

    /// Room Owner Name (for bot user)
    #[arg(long)]
    owner_name: Option<String>,

    /// Public Endpoint URL (e.g. http://example.com/room)
    #[arg(long)]
    endpoint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ServerConfig {
    port: u16,
    host: String,
    app: String,
    room_id: String,
    title: String,
    description: Option<String>,
    owner_id: String,
    owner_name: String,
    endpoint: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "0.0.0.0".to_string(), // Listen on all interfaces by default
            app: "doudizhu".to_string(),
            room_id: "room-1".to_string(),
            title: "OpenPlay Room".to_string(),
            description: Some("A default OpenPlay room".to_string()),
            owner_id: "system".to_string(),
            owner_name: "System".to_string(),
            endpoint: None,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // 1. Parse CLI args to see if config file is specified
    let args = Args::parse();

    // 2. Build Configuration
    let mut builder = Config::builder();

    // A. Start with defaults
    let defaults = ServerConfig::default();
    builder = builder
        .set_default("port", defaults.port)?
        .set_default("host", defaults.host)?
        .set_default("app", defaults.app)?
        .set_default("room_id", defaults.room_id)?
        .set_default("title", defaults.title)?
        .set_default("owner_id", defaults.owner_id)?
        .set_default("owner_name", defaults.owner_name)?;

    if let Some(desc) = defaults.description {
        builder = builder.set_default("description", desc)?;
    }

    // B. Load from config file if provided
    if let Some(config_path) = args.config {
        info!("Loading configuration from {}", config_path);
        builder = builder.add_source(File::with_name(&config_path));
    }

    // C. Load from Environment Variables
    // Prefix: OP_HOST_  (e.g., OP_HOST_PORT=8080)
    builder = builder.add_source(Environment::with_prefix("OP_HOST"));

    // D. Override with CLI args if present
    if let Some(port) = args.port {
        builder = builder.set_override("port", port)?;
    }
    if let Some(host) = args.host {
        builder = builder.set_override("host", host)?;
    }
    if let Some(app) = args.app {
        builder = builder.set_override("app", app)?;
    }
    if let Some(room_id) = args.room_id {
        builder = builder.set_override("room_id", room_id)?;
    }
    if let Some(title) = args.title {
        builder = builder.set_override("title", title)?;
    }
    if let Some(description) = args.description {
        builder = builder.set_override("description", description)?;
    }
    if let Some(owner_id) = args.owner_id {
        builder = builder.set_override("owner_id", owner_id)?;
    }
    if let Some(owner_name) = args.owner_name {
        builder = builder.set_override("owner_name", owner_name)?;
    }
    if let Some(endpoint) = args.endpoint {
        builder = builder.set_override("endpoint", endpoint)?;
    }

    // Finalize Config
    let config: ServerConfig = builder.build()?.try_deserialize()?;

    info!("Configuration loaded: {:?}", config);
    info!("Starting OpenPlay Host ({}) on {}:{}", config.app, config.host, config.port);

    // Initialize Owner
    let owner_id = UserId::from(Bytes::from(config.owner_id.clone()));
    let owner = User::new_robot(config.owner_name, owner_id.clone());

    // Initialize Game (Only Doudizhu supported for now)
    if config.app != "doudizhu" {
        tracing::warn!("Unknown game '{}', defaulting to doudizhu", config.app);
    }
    let game = DouDizhuGame::new(vec![]);

    // Determine Endpoint
    let endpoint = config.endpoint.unwrap_or_else(|| {
        let host_display = if config.host == "0.0.0.0" || config.host == "::" {
            "localhost".to_string()
        } else {
            config.host.clone()
        };
        format!("http://{}:{}/room", host_display, config.port)
    });

    let room_info = RoomInfo {
        id: config.room_id,
        title: config.title,
        description: config.description,
        owner: owner_id,
        endpoint,
        game_config: None,
    };

    let room = Room::new(room_info, owner);

    // Create Room Service
    let service = RoomService::new(game, room);
    let service_handle = service.run();

    // Create Server
    let server = RoomServer::new(service_handle).await;

    // Run HTTP Server
    let addr_str = format!("{}:{}", config.host, config.port);
    let addr = tokio::net::lookup_host(&addr_str)
        .await?
        .next()
        .ok_or_else(|| format!("Failed to resolve address: {}", addr_str))?;

    run_server(addr, server.registry, server.handle.connection_controller).await?;

    Ok(())
}
