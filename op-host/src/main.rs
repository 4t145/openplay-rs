use bytes::Bytes;
use clap::Parser;
use op_host::{run_server, RoomServer};
use openplay_basic::room::{Room, RoomInfo};
use openplay_basic::user::{User, UserId};
use openplay_doudizhu::DouDizhuGame;
use openplay_host::service::RoomService;
use std::net::SocketAddr;
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 3000)]
    port: u16,

    /// Game type (currently only 'doudizhu' is supported)
    #[arg(short, long, default_value = "doudizhu")]
    game: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse args
    let args = Args::parse();

    info!("Starting OpenPlay Host on port {}", args.port);

    // Initialize Owner
    let owner_id = UserId::from(Bytes::from("system"));
    let owner = User::new_robot("System".to_string(), owner_id.clone());

    // Initialize Doudizhu Game
    let game = DouDizhuGame::new(vec![]); 

    let room_info = RoomInfo {
        id: "room-1".to_string(),
        title: "Doudizhu Room".to_string(),
        description: Some("A default doudizhu room".to_string()),
        owner: owner_id,
        endpoint: format!("http://localhost:{}/room", args.port),
        game_config: None,
    };

    let room = Room::new(room_info, owner);

    // Create Room Service
    let service = RoomService::new(game, room);
    let service_handle = service.run();

    // Create Server
    let server = RoomServer::new(service_handle).await;

    // Run HTTP Server
    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    
    run_server(addr, server.registry, server.handle.connection_controller).await?;

    Ok(())
}
