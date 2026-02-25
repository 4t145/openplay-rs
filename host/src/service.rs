use std::collections::HashMap;

use openplay_basic::{
    game::{DynGame, UpdateGameState},
    user::{DynPlayerAgent, UserId, player_event::PlayerEvent},
    room::{
        GameMessage, PlayerBecomeObserver, PlayerBecomePlayer, PlayerChat, PlayerKickedOut,
        PlayerLeave, PlayerReady, PlayerReconnected, Room, RoomEvent, RoomObserverState,
        RoomPlayerState,
    },
};
use tokio_util::sync::CancellationToken;

use crate::connection::PlayerEventWithPid;

pub struct RoomService {
    pub game: DynGame,
    pub room: Room,
    pub player_agents: HashMap<UserId, DynPlayerAgent>,
}

#[derive(Debug, thiserror::Error)]
pub enum RoomServiceError {
    #[error("Connection handle error")]
    ConnectionHandleError(#[from] crate::connection::ConnectionHandleError),
}

pub struct RoomServiceHandle {
    pub cancel_token: CancellationToken,
    pub join_handle: tokio::task::JoinHandle<Result<(), RoomServiceError>>,
}

impl RoomService {
    pub fn from_dyn(game: DynGame, room: Room) -> Self {
        RoomService {
            game,
            room,
            player_agents: HashMap::new(),
        }
    }
    pub fn new<G: openplay_basic::game::Game>(game: G, room: Room) -> Self {
        Self::from_dyn(Box::new(game), room)
    }
    pub fn run(self) -> RoomServiceHandle {
        let Self {
            mut game,
            mut room,
            player_agents,
        } = self;
        let (player_event_tx, mut player_event_rx) =
            tokio::sync::mpsc::channel::<PlayerEventWithPid>(32);
        let ct = CancellationToken::new();
        let handle_ct = ct.clone();
        let task = async move {
            let connection_handle =
                crate::connection::ConnectionHandle::run(player_agents, player_event_tx);
            enum Event {
                PlayerEvent(PlayerEventWithPid),
            }
            loop {
                let evt = tokio::select! {
                    player_event = player_event_rx.recv(), if !player_event_rx.is_closed() && !ct.is_cancelled() => {
                        if let Some(event) = player_event {
                            Event::PlayerEvent(event)
                        } else {
                            // Channel closed, break the loop
                            break;
                        }
                    }
                };
                match evt {
                    Event::PlayerEvent(event) => {
                        let player_id = event.player_id;
                        match event.event {
                            PlayerEvent::PlayerChat(player_chat) => {
                                connection_handle
                                    .broadcast_room_event(RoomEvent::PlayerChat(PlayerChat {
                                        player_id,
                                        message: player_chat.message,
                                    }))
                                    .await?;
                            }
                            PlayerEvent::PlayerReady(player_ready) => {
                                if let Some(player_state) = room
                                    .state
                                    .players
                                    .values_mut()
                                    .find(|state| state.player.id == player_id)
                                {
                                    player_state.id_ready = player_ready.is_ready;
                                    connection_handle
                                        .broadcast_room_event(RoomEvent::PlayerReady(PlayerReady {
                                            player_id,
                                            is_ready: player_ready.is_ready,
                                        }))
                                        .await?;
                                } else {
                                    // Player not found in players, ignore or handle error
                                }
                            }
                            PlayerEvent::KickOut(kick_out) => {
                                if let Some(_player_state) = room.remove_player(&kick_out.player) {
                                    connection_handle
                                        .broadcast_room_event(RoomEvent::PlayerKickedOut(
                                            PlayerKickedOut {
                                                player_id: kick_out.player,
                                            },
                                        ))
                                        .await?;
                                } else {
                                    // Player not found in players, ignore or handle error
                                }
                            }
                            PlayerEvent::HeartBeat => {
                                // TODO: Handle heartbeat, e.g., update player's last active timestamp
                            }
                            PlayerEvent::GameMessage(game_message) => {
                                tracing::debug!("Service processing GameMessage from {}", player_id);
                                let update = game.handle_action(GameMessage {
                                    player_id,
                                    message: game_message.message,
                                });
                                connection_handle
                                    .broadcast_room_event(RoomEvent::UpdateGameState(update))
                                    .await?;
                            }
                            PlayerEvent::StartGame => {
                                tracing::info!("Service received StartGame from {}", player_id);
                                let update = game.start();
                                connection_handle
                                    .broadcast_room_event(RoomEvent::UpdateGameState(update))
                                    .await?;
                            }
                            PlayerEvent::Leave => {
                                if let Some(_player_state) = room.remove_player(&player_id) {
                                    connection_handle
                                        .broadcast_room_event(RoomEvent::PlayerLeave(PlayerLeave {
                                            player_id,
                                        }))
                                        .await?;
                                } else {
                                    // Player not found in players, ignore or handle error
                                }
                            }
                            PlayerEvent::BecomePlayer(become_player) => {
                                if let Some(observer) = room.state.observers.remove(&player_id) {
                                    let player_state = RoomPlayerState {
                                        id_ready: false,
                                        is_connected: observer.is_connected,
                                        player: observer.player.clone(),
                                    };
                                    room.state
                                        .players
                                        .insert(become_player.position.clone(), player_state);
                                    connection_handle
                                        .broadcast_room_event(RoomEvent::PlayerBecomePlayer(
                                            PlayerBecomePlayer {
                                                player_id,
                                                position: become_player.position,
                                            },
                                        ))
                                        .await?;
                                } else {
                                    // Player not found in observers, ignore or handle error
                                }
                            }
                            PlayerEvent::BecomeObserver(become_observer) => {
                                if let Some((position, player_state)) = room
                                    .state
                                    .players
                                    .iter()
                                    .find(|(_, state)| state.player.id == player_id)
                                    .map(|(pos, state)| (pos.clone(), state.clone()))
                                {
                                    room.state.players.remove(&position);
                                    let observer_state = RoomObserverState {
                                        is_connected: player_state.is_connected,
                                        view: become_observer.view.clone(),
                                        player: player_state.player.clone(),
                                    };
                                    room.state
                                        .observers
                                        .insert(player_id.clone(), observer_state);
                                    connection_handle
                                        .broadcast_room_event(RoomEvent::PlayerBecomeObserver(
                                            PlayerBecomeObserver {
                                                player_id,
                                                view: become_observer.view,
                                            },
                                        ))
                                        .await?;
                                } else {
                                    // Player not found in players, ignore or handle error
                                }
                            }
                            PlayerEvent::Reconnect => {
                                connection_handle
                                    .broadcast_room_event(RoomEvent::PlayerReconnected(
                                        PlayerReconnected { player_id },
                                    ))
                                    .await?;
                                connection_handle
                                    .broadcast_room_event(RoomEvent::UpdateGameState(
                                        UpdateGameState::snapshot(game.snapshot()),
                                    ))
                                    .await?;
                            }
                        }
                    }
                }
            }
            <Result<(), RoomServiceError>>::Ok(())
        };
        let join_handle = tokio::spawn(task);

        let handle = RoomServiceHandle {
            cancel_token: handle_ct,
            join_handle,
        };

        handle
    }
}
