mod app;
mod client;
mod config;
mod event;
mod i18n;
mod log_buffer;
mod ui;

use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::app::{App, KeyAction};
use crate::config::{CliArgs, TuiConfig};
use crate::event::EventManager;
use crate::log_buffer::{LogBuffer, LogBufferLayer};

use openplay_basic::user::{
    room_action::{JoinRoom, RoomActionData},
    ActionData,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI args and load config
    let args = CliArgs::parse();
    let cfg = TuiConfig::load(&args)?;

    // Initialize logging into in-memory buffer (displayed in TUI log panel)
    let log_buffer = LogBuffer::new(500);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with(LogBufferLayer::new(log_buffer.clone()))
        .init();

    // Initialize i18n
    i18n::init(cfg.locale.as_deref());

    // Setup terminal
    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Run the app
    let result = run_app(&mut terminal, cfg, log_buffer).await;

    // Restore terminal
    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(ref e) = result {
        eprintln!("Error: {:#}", e);
    }

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    cfg: TuiConfig,
    log_buffer: LogBuffer,
) -> Result<()> {
    let mut app = App::new(cfg.server_url.clone(), cfg.user_id.clone(), log_buffer);
    let mut events = EventManager::new(Duration::from_millis(250));

    // If we have both server_url and user_id from config, auto-connect
    if cfg.user_id.is_some() {
        if let Some((server_url, user_id)) = app.start_connect() {
            match try_connect(&mut app, &server_url, &user_id, &cfg.room_path) {
                Ok(sse_stream) => {
                    events.attach_sse(sse_stream);
                }
                Err(e) => {
                    app.connection_failed(format!("{:#}", e));
                }
            }
        }
    }

    // Main loop
    loop {
        // Draw
        terminal.draw(|f| ui::draw(f, &app))?;

        // Wait for next event
        let Some(evt) = events.next_event().await else {
            break; // Channel closed, exit
        };

        match evt {
            event::AppEvent::Key(key) => {
                match app.handle_key(key) {
                    KeyAction::None => {}
                    KeyAction::Quit => {
                        // Graceful disconnect before quitting
                        if let Some(ref client) = app.client {
                            let _ = client.disconnect().await;
                        }
                        break;
                    }
                    KeyAction::Connect => {
                        if let Some((server_url, user_id)) = app.start_connect() {
                            match try_connect(&mut app, &server_url, &user_id, &cfg.room_path) {
                                Ok(sse_stream) => {
                                    events.attach_sse(sse_stream);
                                }
                                Err(e) => {
                                    app.connection_failed(format!("{:#}", e));
                                }
                            }
                        }
                    }
                    KeyAction::Disconnect => {
                        events.detach_sse();
                        if let Some(ref client) = app.client {
                            let _ = client.disconnect().await;
                        }
                        // From Reconnecting, always go to lobby
                        // From Game, also go to lobby (user explicitly chose to disconnect)
                        app.go_to_lobby(None);
                    }
                    KeyAction::SendAction(action) => {
                        if let Some(ref client) = app.client {
                            if let Err(e) = client.send_action(action).await {
                                tracing::warn!("Failed to send action: {:#}", e);
                            }
                        }
                    }
                }
            }
            event::AppEvent::ServerConnected => {
                tracing::info!("SSE connection confirmed");
                match &app.screen {
                    app::Screen::Connecting => {
                        // Normal first connection
                        let user_id = app.pending_user_id.clone();
                        app.connected();
                        // Auto-send Join action so the server adds us to room.state
                        if let (Some(ref client), Some(uid)) = (&app.client, user_id) {
                            let join_action = ActionData::RoomAction(RoomActionData::Join(JoinRoom {
                                nickname: uid,
                            }));
                            if let Err(e) = client.send_action(join_action).await {
                                tracing::warn!("Failed to send Join action: {:#}", e);
                            }
                        }
                    }
                    app::Screen::Reconnecting(_) => {
                        // Reconnection succeeded — get user_id from the preserved GameState
                        // before transitioning back to Game screen
                        let user_id = if let app::Screen::Reconnecting(ref rs) = app.screen {
                            Some(rs.game_state.my_user_id.clone())
                        } else {
                            None
                        };
                        app.reconnected();
                        // Re-send Join to tell server we're back
                        if let (Some(ref client), Some(uid)) = (&app.client, user_id) {
                            let join_action = ActionData::RoomAction(RoomActionData::Join(JoinRoom {
                                nickname: uid,
                            }));
                            if let Err(e) = client.send_action(join_action).await {
                                tracing::warn!("Failed to send Join action on reconnect: {:#}", e);
                            }
                        }
                    }
                    _ => {}
                }
            }
            event::AppEvent::ServerUpdate(update) => {
                app.handle_server_update(update);
            }
            event::AppEvent::ServerError(err) => {
                tracing::error!("Server error: {}", err);
                match app.screen {
                    // If still connecting, treat error as connection failure
                    app::Screen::Connecting => {
                        events.detach_sse();
                        app.connection_failed(err);
                    }
                    // If in game, show error in message log
                    app::Screen::Game(ref mut gs) => {
                        gs.push_message(format!("Error: {}", err));
                    }
                    // If reconnecting, the SSE task will end and ServerDisconnected
                    // will follow, which triggers the next retry via Tick.
                    // Just record the error for display.
                    app::Screen::Reconnecting(ref mut rs) => {
                        rs.last_error = Some(err);
                    }
                    _ => {}
                }
            }
            event::AppEvent::ServerDisconnected => {
                events.detach_sse();
                // Check if we're in Reconnecting first (needs mutable borrow)
                let action = match &app.screen {
                    app::Screen::Connecting => Some("connection_failed"),
                    app::Screen::Game(gs) if gs.game.is_some() => Some("reconnect"),
                    app::Screen::Game(_) => Some("lobby"),
                    app::Screen::Reconnecting(_) => Some("retry"),
                    _ => None,
                };
                match action {
                    Some("connection_failed") => {
                        app.connection_failed("Connection closed before completing".to_string());
                    }
                    Some("reconnect") => {
                        let server_url = cfg.server_url.clone();
                        let room_path = cfg.room_path.clone();
                        app.go_to_reconnecting(server_url, room_path);
                        tracing::info!("Entering reconnection mode");
                    }
                    Some("lobby") => {
                        app.go_to_lobby(Some("Server disconnected".to_string()));
                    }
                    Some("retry") => {
                        // A reconnection attempt's SSE stream ended.
                        // Increment attempts; the Tick handler will retry.
                        if let app::Screen::Reconnecting(ref mut rs) = app.screen {
                            rs.attempts += 1;
                            if rs.last_error.is_none() {
                                rs.last_error = Some("Connection closed".to_string());
                            }
                            tracing::warn!(
                                "Reconnect attempt {} failed",
                                rs.attempts
                            );
                        }
                    }
                    _ => {}
                }
            }
            event::AppEvent::Tick => {
                // Drive reconnection attempts
                if let app::Screen::Reconnecting(ref rs) = app.screen {
                    // Check if we've exhausted all attempts
                    if rs.attempts >= rs.max_attempts {
                        tracing::error!("Max reconnection attempts reached, returning to lobby");
                        app.go_to_lobby(Some("Reconnection failed after max attempts".to_string()));
                    } else if app.client.is_none() {
                        // No active connection attempt — start one
                        let server_url = rs.server_url.clone();
                        let room_path = rs.room_path.clone();
                        let user_id = rs.game_state.my_user_id.clone();
                        tracing::info!(
                            "Reconnection attempt {}/{}",
                            rs.attempts + 1,
                            rs.max_attempts,
                        );
                        match try_connect(&mut app, &server_url, &user_id, &room_path) {
                            Ok(sse_stream) => {
                                events.attach_sse(sse_stream);
                            }
                            Err(e) => {
                                tracing::warn!("Reconnect try_connect failed: {:#}", e);
                                if let app::Screen::Reconnecting(ref mut rs) = app.screen {
                                    rs.last_error = Some(format!("{:#}", e));
                                    rs.attempts += 1;
                                }
                            }
                        }
                    }
                    // else: client exists, SSE is in-flight — wait for ServerConnected or ServerDisconnected
                }
            }
            event::AppEvent::Resize(_, _) => {
                // Terminal resize, just redraw
            }
        }
    }

    Ok(())
}

/// Create a GameClient and start its SSE stream (lazy — no actual HTTP yet).
/// Sets `app.client` but does NOT change screen state (that happens on ServerConnected).
fn try_connect(
    app: &mut App,
    server_url: &str,
    user_id: &str,
    room_path: &str,
) -> Result<impl futures::Stream<Item = Result<client::SseEvent>> + Send + 'static> {
    let client = client::GameClient::new(
        server_url.to_string(),
        room_path.to_string(),
        user_id.to_string(),
    )?;

    let sse_stream = client.connect_sse();
    app.client = Some(client);

    Ok(sse_stream)
}
