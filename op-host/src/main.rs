use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use bytes::Bytes;
use clap::Parser;
use config::{Config, Environment, File};
use op_host::{run_server, RoomServer};
use openplay_basic::data::Data;
use openplay_basic::game::GameViewUpdate;
use openplay_basic::message::{DataType, TypedData};
use openplay_basic::room::{Room, RoomInfo};
use openplay_basic::user::{new_dyn_user_agent, DynUserAgent, User, UserId};
use openplay_doudizhu::DouDizhuGame;
use openplay_host::service::{BotFactory, RoomService};
use openplay_ua_programmed::{ProgrammedUserAgent, UserProgram};
use serde::Deserialize;
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

    /// Room Owner Display Name
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
            owner_id: "alice".to_string(),
            owner_name: "Alice".to_string(),
            endpoint: None,
        }
    }
}

// --- Doudizhu Bot Infrastructure ---

/// A `UserProgram` that uses `SimpleBotLogic` to decide doudizhu actions.
struct DoudizhuBotProgram {
    player_id: UserId,
}

impl UserProgram for DoudizhuBotProgram {
    fn decide(&self, update: &GameViewUpdate) -> Option<TypedData> {
        let game: DouDizhuGame = match serde_json::from_slice(&update.new_view.data.data.0) {
            Ok(g) => g,
            Err(e) => {
                tracing::warn!("Bot failed to decode game state: {}", e);
                return None;
            }
        };

        if matches!(game.stage, openplay_doudizhu::Stage::Finished) {
            return None;
        }

        if let Some(action) = openplay_doudizhu::bot::SimpleBotLogic::decide(&self.player_id, &game) {
            let json = serde_json::to_string(&action).ok()?;
            Some(TypedData {
                r#type: DataType {
                    app: openplay_doudizhu::get_app(),
                    r#type: "action".to_string(),
                },
                codec: "json".to_string(),
                data: Data(Bytes::from(json)),
            })
        } else {
            None
        }
    }
}

/// Factory that creates doudizhu bot agents.
struct DoudizhuBotFactory {
    bot_counter: AtomicU32,
}

impl DoudizhuBotFactory {
    fn new() -> Self {
        Self {
            bot_counter: AtomicU32::new(1),
        }
    }
}

impl BotFactory for DoudizhuBotFactory {
    fn create_bot(&self, name: Option<String>) -> (User, DynUserAgent) {
        let n = self.bot_counter.fetch_add(1, Ordering::Relaxed);
        let bot_name = name.unwrap_or_else(|| format!("Bot-{}", n));
        let bot_id = UserId::from(Bytes::from(format!("bot-{}", n)));

        let bot_user = User::new_robot(bot_name, bot_id.clone());
        let program = DoudizhuBotProgram {
            player_id: bot_id,
        };
        let agent = ProgrammedUserAgent::new(bot_user.clone(), program);
        let dyn_agent = new_dyn_user_agent(agent);

        (bot_user, dyn_agent)
    }
}

// --- Main ---

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

    // Initialize Owner (real user, not a bot — TUI user connects with the same owner_id)
    let owner_id = UserId::from(Bytes::from(config.owner_id.clone()));
    let owner = User {
        nickname: config.owner_name,
        id: owner_id.clone(),
        avatar_url: None,
        is_bot: false,
    };

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

    // Create Room Service with bot factory
    let bot_factory = Arc::new(DoudizhuBotFactory::new());
    let service = RoomService::new(game, room).with_bot_factory(bot_factory);
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
