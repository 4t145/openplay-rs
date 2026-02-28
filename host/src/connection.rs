use std::collections::HashMap;

use openplay_basic::{
    game::GameViewUpdate,
    room::{RoomUpdate, Update},
    user::{Action, DynUserAgent, UserId},
};
use tokio_util::sync::CancellationToken;

// type Responder<T, E> = (T, tokio::sync::oneshot::Sender<Result<(), E>>);
type ResultResponder<T, E> = tokio::sync::oneshot::Sender<Result<T, E>>;
type Responder<T> = tokio::sync::oneshot::Sender<T>;
pub enum ConnectionCommand {
    Connect {
        user_id: UserId,
        agent: DynUserAgent,
        responder: Responder<()>,
    },
    Disconnect {
        user_id: UserId,
        responder: Responder<Option<UaProxyQuitReason>>,
    },
    RoomUpdate(RoomUpdate),
    GameViewUpdate {
        update: GameViewUpdate,
        to: UserId,
    },
}

pub struct ConnectionBroadcastMessage {
    update: Update,
}
pub struct ConnectionContext {
    pub user_agents: HashMap<UserId, DynUserAgent>,
    pub command_rx: tokio::sync::mpsc::Receiver<ConnectionCommand>,
}

pub struct ConnectionController {
    command_tx: tokio::sync::mpsc::Sender<ConnectionCommand>,
}

impl Clone for ConnectionController {
    fn clone(&self) -> Self {
        Self {
            command_tx: self.command_tx.clone(),
        }
    }
}

pub struct ConnectionHandle {
    ct: CancellationToken,
    task_handle: tokio::task::JoinHandle<()>,
}

pub enum ConnectionEvent {
    UserDisconnected {
        user_id: UserId,
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

impl ConnectionController {
    pub async fn user_connect(
        &self,
        user_id: UserId,
        agent: DynUserAgent,
    ) -> Result<(), ConnectionHandleError> {
        let (responder_tx, responder_rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(ConnectionCommand::Connect {
                user_id: user_id,
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
    pub async fn user_disconnect(
        &self,
        user_id: UserId,
    ) -> Result<Option<UaProxyQuitReason>, ConnectionHandleError> {
        let (responder_tx, responder_rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(ConnectionCommand::Disconnect {
                user_id: user_id,
                responder: responder_tx,
            })
            .await
            .map_err(|_| ConnectionHandleError::CommandSendError)?;
        let response = responder_rx
            .await
            .map_err(|_| ConnectionHandleError::CommandResponseError)?;
        Ok(response)
    }
    pub async fn broadcast_room_update(
        &self,
        update: RoomUpdate,
    ) -> Result<(), ConnectionHandleError> {
        self.command_tx
            .send(ConnectionCommand::RoomUpdate(update))
            .await
            .map_err(|_| ConnectionHandleError::CommandSendError)
    }
    pub async fn send_game_view_update(
        &self,
        update: GameViewUpdate,
        to: UserId,
    ) -> Result<(), ConnectionHandleError> {
        self.command_tx
            .send(ConnectionCommand::GameViewUpdate { update, to })
            .await
            .map_err(|_| ConnectionHandleError::CommandSendError)
    }
}

impl ConnectionHandle {
    pub async fn quit(self) {
        self.ct.cancel();
        let _ = self.task_handle.await;
    }

    pub fn run(
        user_agents: impl IntoIterator<Item = (UserId, DynUserAgent)> + Send + 'static,
        user_event_tx: tokio::sync::mpsc::Sender<Action>,
    ) -> (ConnectionHandle, ConnectionController) {
        let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::channel::<ConnectionCommand>(32);
        let ct = CancellationToken::new();
        let handle_ct = ct.clone();
        let task = async move {
            let mut agents: HashMap<UserId, UaProxyHandle> = user_agents
                .into_iter()
                .map(|(user_id, agent)| {
                    let handle = UaProxyHandle::run(UaProxyContext {
                        user_id: user_id.clone(),
                        agent,
                        user_event_tx: user_event_tx.clone(),
                    });
                    (user_id, handle)
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
                        user_id,
                        agent,
                        responder,
                    }) => {
                        tracing::info!("ConnectionHandle: User {} connecting", user_id);
                        // Handle new connection, e.g., by creating a new PlayerProxyHandle and adding it to the agents map.
                        let handle = UaProxyHandle::run(UaProxyContext {
                            user_id: user_id.clone(),
                            agent,
                            user_event_tx: user_event_tx.clone(),
                        });
                        agents.insert(user_id, handle);
                        let _ = responder.send(());
                    }
                    Event::ReceiveCmd(ConnectionCommand::Disconnect { user_id, responder }) => {
                        tracing::info!("ConnectionHandle: User {} disconnecting", user_id);
                        // Handle disconnection, e.g., by removing the PlayerProxyHandle from the agents map and quitting it.
                        let reason = if let Some(handle) = agents.remove(&user_id) {
                            let reason = handle.quit().await;
                            Some(reason)
                        } else {
                            None
                        };
                        let _ = responder.send(reason);
                    }
                    Event::ReceiveCmd(ConnectionCommand::RoomUpdate(update)) => {
                        tracing::debug!("ConnectionHandle broadcasting Update: {:?}", update);
                        let mut batch_send_join_set = tokio::task::JoinSet::new();
                        for (_pid, handle) in &agents {
                            let update = update.clone();
                            let send_task = handle.send_update(Update::Room(update));
                            let user_id = handle.user_id.clone();
                            batch_send_join_set.spawn(async move {
                                if let Err(e) = send_task.await {
                                    tracing::error!(
                                        "Error sending room update to user {}: {:?}",
                                        user_id,
                                        e
                                    );
                                }
                            });
                        }
                        // Detach tasks so they continue running even if JoinSet is dropped?
                        // JoinSet::detach_all()
                        batch_send_join_set.detach_all();
                    }
                    Event::ReceiveCmd(ConnectionCommand::GameViewUpdate { update, to }) => {
                        tracing::debug!(
                            "ConnectionHandle sending GameViewUpdate to {}: {:?}",
                            to,
                            update
                        );
                        if let Some(handle) = agents.get(&to) {
                            let send_task = handle.send_update(Update::GameView(update));
                            let user_id = handle.user_id.clone();
                            tokio::spawn(async move {
                                if let Err(e) = send_task.await {
                                    tracing::error!(
                                        "Error sending game view update to user {}: {:?}",
                                        user_id,
                                        e
                                    );
                                }
                            });
                        } else {
                            tracing::warn!(
                                "ConnectionHandle: User {} not found for GameViewUpdate",
                                to
                            );
                        }
                    }
                }
            }
        };
        let handle = ConnectionHandle {
            ct: handle_ct,
            task_handle: tokio::spawn(task),
        };
        let controller = ConnectionController { command_tx: cmd_tx };
        (handle, controller)
    }
}

pub struct UaProxyContext {
    user_id: UserId,
    agent: DynUserAgent,
    user_event_tx: tokio::sync::mpsc::Sender<Action>,
}

#[derive(Debug, thiserror::Error)]
#[error("UaProxy internal error for user {user_id}: {internal_error:?}")]
pub struct UaProxyError {
    user_id: UserId,
    internal_error: Option<Box<dyn std::error::Error + Send + Sync>>,
}

#[derive(Debug, thiserror::Error)]
pub enum UaProxyQuitReason {
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

pub struct UaProxyHandle {
    user_id: UserId,
    tokio_handle: tokio::task::JoinHandle<UaProxyQuitReason>,
    cancellation_token: tokio_util::sync::CancellationToken,
    room_event_tx: tokio::sync::mpsc::Sender<(Update, ResultResponder<(), UaProxyError>)>,
}

impl UaProxyHandle {
    pub async fn quit(self) -> UaProxyQuitReason {
        self.cancellation_token.cancel();
        self.tokio_handle
            .await
            .expect("PlayerProxyHandle tokio join error")
    }
    pub fn send_update(
        &self,
        update: Update,
    ) -> impl Future<Output = Result<(), UaProxyError>> + Send + 'static + use<> {
        let event_tx = self.room_event_tx.clone();
        let user_id = self.user_id.clone();
        async move {
            let (responder_tx, responder_rx) = tokio::sync::oneshot::channel();
            event_tx
                .send((update, responder_tx))
                .await
                .map_err(|_| UaProxyError {
                    user_id: user_id.clone(), // Use the actual player ID.
                    internal_error: None,
                })?;
            responder_rx.await.map_err(|_| UaProxyError {
                user_id: user_id.clone(), // Use the actual player ID.
                internal_error: None,
            })??;
            Ok(())
        }
    }
    pub fn run(context: UaProxyContext) -> UaProxyHandle {
        let UaProxyContext {
            user_id,
            agent,
            user_event_tx,
        } = context;
        let (proxy_room_event_tx, proxy_user_event_rx) =
            tokio::sync::mpsc::channel::<(Update, ResultResponder<(), UaProxyError>)>(32);
        let ct = tokio_util::sync::CancellationToken::new();
        let handle_ct = ct.clone();
        enum Event {
            ReceiveUserAction(Action),
            SendUpdate(Update, ResultResponder<(), UaProxyError>),
        }
        let mut message_rx = proxy_user_event_rx;
        let message_tx = user_event_tx;
        let handle_user_id = user_id.clone();
        let task = async move {
            let quit_reason = loop {
                // println!("UaProxy for {} loop start", handle_user_id);
                let evt = tokio::select! {
                    biased; // Try biased to check if order matters, though usually random
                    _ = ct.cancelled() => {
                        break UaProxyQuitReason::Cancelled;
                    }
                    responder = message_rx.recv(), if !message_rx.is_closed() && !ct.is_cancelled() => {
                        match responder {
                            Some((update,responder)) => Event::SendUpdate(update, responder),
                            None => {
                                // The sender has been dropped, which means the connection is closed.
                                break UaProxyQuitReason::SenderClosed;
                            }
                        }
                    }
                    action = agent.receive_action(), if !ct.is_cancelled() => {
                        match action {
                            Ok(Some(user_action)) => Event::ReceiveUserAction(user_action.with_source(&user_id)),
                            Ok(None) => {
                                // The agent has closed the connection.
                                break UaProxyQuitReason::ReceiverClosed;
                            }
                            Err(e) => {
                                break UaProxyQuitReason::ReceiverError(e);
                            }
                        }
                    }
                };
                match evt {
                    Event::ReceiveUserAction(user_action) => {
                        // Forward the user action to the connection.
                        if let Err(_) = message_tx.send(user_action).await {
                            // server closed
                            break UaProxyQuitReason::ServerClosed;
                        }
                    }
                    Event::SendUpdate(update, responder) => {
                        // Forward the room event to the agent.
                        let forward_result = agent.send_update(update).await;
                        let _ =
                            responder.send(forward_result.map_err(|internal_error| UaProxyError {
                                user_id: user_id.clone(),
                                internal_error: Some(internal_error),
                            }));
                    }
                }
            };
            tracing::info!(
                "UserAgentProxy for user {} disconnected: {:?}",
                user_id,
                quit_reason
            );
            return quit_reason;
        };
        let tokio_handle = tokio::spawn(task);
        UaProxyHandle {
            user_id: handle_user_id,
            tokio_handle,
            cancellation_token: handle_ct,
            room_event_tx: proxy_room_event_tx,
        }
    }
}
