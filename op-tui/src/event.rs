use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self as ct_event, Event as CrosstermEvent, KeyEvent};
use futures::stream::{Stream, StreamExt};
use openplay_basic::room::Update;
use tokio::sync::mpsc;

use openplay_client::SseEvent;

/// Unified application event, merging terminal input, server updates, and ticks.
#[derive(Debug)]
pub enum AppEvent {
    /// Terminal key press
    Key(KeyEvent),
    /// Terminal resize
    #[allow(dead_code)]
    Resize(u16, u16),
    /// SSE connection successfully opened
    ServerConnected,
    /// Server sent an update via SSE
    ServerUpdate(Update),
    /// Server connection error
    ServerError(String),
    /// Server SSE stream ended
    ServerDisconnected,
    /// Periodic tick for UI refresh
    Tick,
}

/// Manages the unified event channel.
/// Terminal input and tick are spawned once and live for the entire app lifetime.
/// SSE streams can be swapped in/out as connections come and go.
pub struct EventManager {
    rx: mpsc::UnboundedReceiver<AppEvent>,
    tx: mpsc::UnboundedSender<AppEvent>,
    /// Handle to the current SSE task so we can abort it when reconnecting.
    sse_task: Option<tokio::task::JoinHandle<()>>,
}

impl EventManager {
    /// Create a new EventManager. Spawns terminal input reader and tick timer immediately.
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        // Terminal input reader — spawned once, lives forever
        {
            let tx = tx.clone();
            tokio::spawn(async move {
                loop {
                    // Poll crossterm events with a timeout so we yield to tokio periodically
                    let available = tokio::task::spawn_blocking(|| {
                        ct_event::poll(Duration::from_millis(50))
                    })
                    .await;

                    match available {
                        Ok(Ok(true)) => {
                            let event = tokio::task::spawn_blocking(ct_event::read).await;
                            match event {
                                Ok(Ok(CrosstermEvent::Key(key))) => {
                                    if tx.send(AppEvent::Key(key)).is_err() {
                                        break;
                                    }
                                }
                                Ok(Ok(CrosstermEvent::Resize(w, h))) => {
                                    if tx.send(AppEvent::Resize(w, h)).is_err() {
                                        break;
                                    }
                                }
                                Ok(Ok(_)) => {} // Ignore mouse, focus, paste events
                                Ok(Err(_)) => break,
                                Err(_) => break,
                            }
                        }
                        Ok(Ok(false)) => {} // No event ready, loop around
                        _ => break,
                    }
                }
            });
        }

        // Tick timer — spawned once, lives forever
        {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(tick_rate);
                loop {
                    interval.tick().await;
                    if tx.send(AppEvent::Tick).is_err() {
                        break;
                    }
                }
            });
        }

        Self {
            rx,
            tx,
            sse_task: None,
        }
    }

    /// Attach an SSE stream. Aborts any previous SSE task first.
    pub fn attach_sse(
        &mut self,
        sse_stream: impl Stream<Item = Result<SseEvent>> + Send + 'static,
    ) {
        // Abort previous SSE task if any
        if let Some(handle) = self.sse_task.take() {
            handle.abort();
        }

        let tx = self.tx.clone();
        let handle = tokio::spawn(async move {
            let mut stream = std::pin::pin!(sse_stream);
            while let Some(result) = stream.next().await {
                let event = match result {
                    Ok(SseEvent::Connected) => AppEvent::ServerConnected,
                    Ok(SseEvent::Update(update)) => AppEvent::ServerUpdate(update),
                    Err(e) => AppEvent::ServerError(e.to_string()),
                };
                if tx.send(event).is_err() {
                    break;
                }
            }
            let _ = tx.send(AppEvent::ServerDisconnected);
        });

        self.sse_task = Some(handle);
    }

    /// Detach the current SSE stream (aborts the task, no ServerDisconnected event sent).
    pub fn detach_sse(&mut self) {
        if let Some(handle) = self.sse_task.take() {
            handle.abort();
        }
    }

    /// Receive the next event. Blocks until an event is available.
    pub async fn next_event(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }
}
