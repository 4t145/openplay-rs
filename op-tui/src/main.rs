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
                        // Disconnect from server and return to lobby
                        events.detach_sse();
                        if let Some(ref client) = app.client {
                            let _ = client.disconnect().await;
                        }
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
                // Grab user_id before connected() consumes pending_user_id
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
                    _ => {}
                }
            }
            event::AppEvent::ServerDisconnected => {
                events.detach_sse();
                match app.screen {
                    app::Screen::Connecting => {
                        app.connection_failed("Connection closed before completing".to_string());
                    }
                    app::Screen::Game(_) => {
                        app.go_to_lobby(Some("Server disconnected".to_string()));
                    }
                    _ => {}
                }
            }
            event::AppEvent::Tick => {
                // Just triggers a redraw via the loop
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
