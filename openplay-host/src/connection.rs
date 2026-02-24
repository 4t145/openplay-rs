use std::collections::HashMap;

use openplay_basic::{
    game::DynGame,
    player::{
        DynPlayerAgent, PlayerId,
        player_event::{self, PlayerEvent},
    },
    room::{Room, RoomEvent},
};
use tokio_util::sync::CancellationToken;

// type Responder<T, E> = (T, tokio::sync::oneshot::Sender<Result<(), E>>);
type ResultResponder<T, E> = tokio::sync::oneshot::Sender<Result<T, E>>;
type Responder<T> = tokio::sync::oneshot::Sender<T>;
pub enum ConnectionCommand {
    Connect {
        player_id: PlayerId,
        agent: DynPlayerAgent,
        responder: Responder<()>,
    },
    Disconnect {
        player_id: PlayerId,
        responder: Responder<Option<PlayerAgentProxyQuitReason>>,
    },
    RoomEvent(RoomEvent),
}

pub struct ConnectionBroadcastMessage {
    room_event: RoomEvent,
}
pub struct ConnectionContext {
    pub player_agents: HashMap<PlayerId, DynPlayerAgent>,
    pub command_rx: tokio::sync::mpsc::Receiver<ConnectionCommand>,
}

pub struct ConnectionHandle {
    command_tx: tokio::sync::mpsc::Sender<ConnectionCommand>,
    ct: CancellationToken,
    task_handle: tokio::task::JoinHandle<()>,
}

pub enum ConnectionEvent {
    PlayerDisconnected {
        player_id: PlayerId,
        error: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}
#[derive(Debug, thiserror::Error)]
pub enum ConnectionHandleError {
    #[error("Command send error")]
    CommandSendError,
    #[error("Command response error")]
    CommandResponseError,
}
impl ConnectionHandle {
    pub async fn quit(self) {
        self.ct.cancel();
        let _ = self.task_handle.await;
    }
    pub async fn player_connect(
        &self,
        player_id: PlayerId,
        agent: DynPlayerAgent,
    ) -> Result<(), ConnectionHandleError> {
        let (responder_tx, responder_rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(ConnectionCommand::Connect {
                player_id,
                agent,
                responder: responder_tx,
            })
            .await
            .map_err(|_| ConnectionHandleError::CommandSendError)?;
        responder_rx
            .await
            .map_err(|_| ConnectionHandleError::CommandResponseError)?;
        Ok(())
    }
    pub async fn player_disconnect(
        &self,
        player_id: PlayerId,
    ) -> Result<Option<PlayerAgentProxyQuitReason>, ConnectionHandleError> {
        let (responder_tx, responder_rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(ConnectionCommand::Disconnect {
                player_id,
                responder: responder_tx,
            })
            .await
            .map_err(|_| ConnectionHandleError::CommandSendError)?;
        let response = responder_rx
            .await
            .map_err(|_| ConnectionHandleError::CommandResponseError)?;
        Ok(response)
    }
    pub async fn broadcast_room_event(
        &self,
        room_event: RoomEvent,
    ) -> Result<(), ConnectionHandleError> {
        self.command_tx
            .send(ConnectionCommand::RoomEvent(room_event))
            .await
            .map_err(|_| ConnectionHandleError::CommandSendError)
    }
    pub fn run(
        player_agents: impl IntoIterator<Item = (PlayerId, DynPlayerAgent)> + Send + 'static,
        player_event_tx: tokio::sync::mpsc::Sender<PlayerEventWithPid>,
    ) -> ConnectionHandle {
        let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::channel::<ConnectionCommand>(32);
        let ct = CancellationToken::new();
        let handle_ct = ct.clone();
        let task = async move {
            let mut agents: HashMap<PlayerId, PlayerProxyHandle> = player_agents
                .into_iter()
                .map(|(player_id, agent)| {
                    let handle = PlayerProxyHandle::run(PlayerProxyContext {
                        player_id: player_id.clone(),
                        agent,
                        player_event_tx: player_event_tx.clone(),
                    });
                    (player_id, handle)
                })
                .collect();
            enum Event {
                ReceiveCmd(ConnectionCommand),
            }
            loop {
                let evt = tokio::select! {
                    _ = ct.cancelled() => {
                        break;
                    }
                    cmd = cmd_rx.recv(), if !cmd_rx.is_closed() && !ct.is_cancelled() => {
                        if let Some(cmd) = cmd {
                            Event::ReceiveCmd(cmd)
                        } else {
                            // The sender has been dropped, which means the connection is closed.
                            break;
                        }
                    }
                };
                match evt {
                    Event::ReceiveCmd(ConnectionCommand::Connect {
                        player_id,
                        agent,
                        responder,
                    }) => {
                        // Handle new connection, e.g., by creating a new PlayerProxyHandle and adding it to the agents map.
                        let handle = PlayerProxyHandle::run(PlayerProxyContext {
                            player_id: player_id.clone(),
                            agent,
                            player_event_tx: player_event_tx.clone(),
                        });
                        agents.insert(player_id, handle);
                        let _ = responder.send(());
                    }
                    Event::ReceiveCmd(ConnectionCommand::Disconnect {
                        player_id,
                        responder,
                    }) => {
                        // Handle disconnection, e.g., by removing the PlayerProxyHandle from the agents map and quitting it.
                        let reason = if let Some(handle) = agents.remove(&player_id) {
                            let reason = handle.quit().await;
                            Some(reason)
                        } else {
                            None
                        };
                        let _ = responder.send(reason);
                    }
                    Event::ReceiveCmd(ConnectionCommand::RoomEvent(room_event)) => {
                        let mut batch_send_join_set = tokio::task::JoinSet::new();
                        for handle in agents.values() {
                            let room_event = room_event.clone();
                            let send_task = handle.send_room_event(room_event);
                            batch_send_join_set.spawn(send_task);
                        }
                    }
                }
            }
        };
        let handle = Self {
            command_tx: cmd_tx,
            ct: handle_ct,
            task_handle: tokio::spawn(task),
        };
        handle
    }
}

pub struct PlayerProxyContext {
    player_id: PlayerId,
    agent: DynPlayerAgent,
    player_event_tx: tokio::sync::mpsc::Sender<PlayerEventWithPid>,
}

pub struct PlayerEventWithPid {
    pub player_id: PlayerId,
    pub event: PlayerEvent,
}

#[derive(Debug, thiserror::Error)]
#[error("PlayerAgentProxy internal error for player {player_id}: {internal_error:?}")]
pub struct PlayerAgentProxyError {
    player_id: PlayerId,
    internal_error: Option<Box<dyn std::error::Error + Send + Sync>>,
}

#[derive(Debug, thiserror::Error)]
pub enum PlayerAgentProxyQuitReason {
    #[error("Sender closed")]
    SenderClosed,
    #[error("Receiver closed")]
    ReceiverClosed,
    #[error("Receiver error: {0}")]
    ReceiverError(Box<dyn std::error::Error + Send + Sync>),
    #[error("Server closed")]
    ServerClosed,
    #[error("Cancelled")]
    Cancelled,
}

pub struct PlayerProxyHandle {
    player_id: PlayerId,
    tokio_handle: tokio::task::JoinHandle<PlayerAgentProxyQuitReason>,
    cancellation_token: tokio_util::sync::CancellationToken,
    room_event_tx:
        tokio::sync::mpsc::Sender<(RoomEvent, ResultResponder<(), PlayerAgentProxyError>)>,
}

impl PlayerProxyHandle {
    pub async fn quit(self) -> PlayerAgentProxyQuitReason {
        self.cancellation_token.cancel();
        self.tokio_handle
            .await
            .expect("PlayerProxyHandle tokio join error")
    }
    pub fn send_room_event(
        &self,
        event: RoomEvent,
    ) -> impl Future<Output = Result<(), PlayerAgentProxyError>> + Send + 'static {
        let event_tx = self.room_event_tx.clone();
        let player_id = self.player_id.clone();
        async move {
            let (responder_tx, responder_rx) = tokio::sync::oneshot::channel();
            event_tx
                .send((event, responder_tx))
                .await
                .map_err(|_| PlayerAgentProxyError {
                    player_id: player_id.clone(), // Use the actual player ID.
                    internal_error: None,
                })?;
            responder_rx.await.map_err(|_| PlayerAgentProxyError {
                player_id: player_id.clone(), // Use the actual player ID.
                internal_error: None,
            })??;
            Ok(())
        }
    }
    pub fn run(context: PlayerProxyContext) -> PlayerProxyHandle {
        let PlayerProxyContext {
            player_id,
            agent,
            player_event_tx,
        } = context;
        let (proxy_room_event_tx, proxy_player_event_rx) = tokio::sync::mpsc::channel::<(
            RoomEvent,
            ResultResponder<(), PlayerAgentProxyError>,
        )>(32);
        let ct = tokio_util::sync::CancellationToken::new();
        let handle_ct = ct.clone();
        enum Event {
            ReceivePlayerEvent(PlayerEvent),
            SendRoomEvent(RoomEvent, ResultResponder<(), PlayerAgentProxyError>),
        }
        let mut message_rx = proxy_player_event_rx;
        let message_tx = player_event_tx;
        let handle_player_id = player_id.clone();
        let task = async move {
            let quit_reason = loop {
                let evt = tokio::select! {
                    _ = ct.cancelled() => {
                        break PlayerAgentProxyQuitReason::Cancelled;
                    }
                    responder = message_rx.recv(), if !message_rx.is_closed() && !ct.is_cancelled() => {
                        match responder {
                            Some((event,responder)) => Event::SendRoomEvent(event, responder),
                            None => {
                                // The sender has been dropped, which means the connection is closed.
                                break PlayerAgentProxyQuitReason::SenderClosed;
                            }
                        }
                    }
                    player_event = agent.receive_player_event(), if !ct.is_cancelled() => {
                        match player_event {
                            Ok(Some(player_event)) => Event::ReceivePlayerEvent(player_event),
                            Ok(None) => {
                                // The agent has closed the connection.
                                break PlayerAgentProxyQuitReason::ReceiverClosed;
                            }
                            Err(e) => {
                                break PlayerAgentProxyQuitReason::ReceiverError(e);
                            }
                        }
                    }
                };
                match evt {
                    Event::ReceivePlayerEvent(player_event) => {
                        // Forward the player event to the connection.
                        if let Err(_) = message_tx
                            .send(PlayerEventWithPid {
                                player_id: player_id.clone(),
                                event: player_event,
                            })
                            .await
                        {
                            // server closed
                            break PlayerAgentProxyQuitReason::ServerClosed;
                        }
                    }
                    Event::SendRoomEvent(event, responder) => {
                        // Forward the room event to the agent.
                        let forward_result = agent.send_room_event(event).await;
                        let _ = responder.send(forward_result.map_err(|internal_error| {
                            PlayerAgentProxyError {
                                player_id: player_id.clone(),
                                internal_error: Some(internal_error),
                            }
                        }));
                    }
                }
            };
            tracing::info!(
                "PlayerAgentProxy for player {} disconnected: {:?}",
                player_id,
                quit_reason
            );
            return quit_reason;
        };
        let tokio_handle = tokio::spawn(task);
        PlayerProxyHandle {
            player_id: handle_player_id,
            tokio_handle,
            cancellation_token: handle_ct,
            room_event_tx: proxy_room_event_tx,
        }
    }
}
