use std::collections::HashMap;

use openplay_basic::{
    game::{
        DynGame, GameCommand, GameEvent, GameUpdate, GameViewUpdate, Interval, SequencedGameUpdate,
        TimeExpired,
    },
    room::{
        Chat, Room, RoomEvent, RoomObserverState, RoomPlayerState, RoomUpdate, RoomUserPosition,
        Update, UserActionEvent,
    },
    user::{
        Action, ActionSource, DynUserAgent, UserId,
        game_action::GameActionData,
        room_action::{PositionChange, ReadyStateChange, RoomActionData, RoomManage},
    },
};
use tokio::task::AbortHandle;
use tokio_util::sync::CancellationToken;

use crate::connection::ConnectionHandle;

pub struct RoomService {
    pub game: DynGame,
    pub room: Room,
    pub player_agents: HashMap<UserId, DynUserAgent>,
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

enum ServiceEvent {
    Action(Action),
    TimerExpired(openplay_basic::game::Id),
    Interval(openplay_basic::game::Id),
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

        let (action_tx, mut action_rx) = tokio::sync::mpsc::channel::<Action>(32);
        let (timer_tx, mut timer_rx) = tokio::sync::mpsc::channel::<ServiceEvent>(32);

        // Forward user actions to the main event loop
        let action_tx_clone = action_tx.clone();
        let timer_tx_clone = timer_tx.clone();
        tokio::spawn(async move {
            while let Some(action) = action_rx.recv().await {
                if timer_tx_clone
                    .send(ServiceEvent::Action(action))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        let ct = CancellationToken::new();
        let handle_ct = ct.clone();

        let task = async move {
            let connection_handle = ConnectionHandle::run(player_agents, action_tx_clone);
            let mut timers: HashMap<openplay_basic::game::Id, AbortHandle> = HashMap::new();

            loop {
                let evt = tokio::select! {
                    event = timer_rx.recv(), if !timer_rx.is_closed() && !ct.is_cancelled() => {
                        match event {
                            Some(evt) => evt,
                            None => break,
                        }
                    }
                    _ = ct.cancelled() => break,
                };

                // Filter non-game events first
                if let ServiceEvent::Action(action) = &evt {
                    if let openplay_basic::user::ActionData::RoomAction(room_action) = &action.data {
                         Self::handle_room_action(
                            &mut room,
                            &connection_handle,
                            action.source(),
                            room_action.clone(),
                        )
                        .await?;

                        // Check for Game Start Condition (All Ready)
                        // Simplified logic: If ready state changed and all players (min 3) are ready -> Start Game
                        // In production this might need a more robust state machine (e.g. checks if game already running)
                        if let openplay_basic::user::room_action::RoomActionData::ChangeReadyState(_) = room_action {
                             if room.state.player_count() >= 3 && room.state.players.values().all(|p| p.id_ready) {
                                  // Trigger Game Start
                                  let update = game.handle_action(
                                      &openplay_basic::room::RoomContext {}, 
                                      SequencedGameUpdate { 
                                          event: GameEvent::GameStart, 
                                          seq: 0 
                                      }
                                  );
                                  
                                  // Handle Game Commands
                                  for command in update.commands {
                                      match command {
                                          GameCommand::CreateTimer { id, duration } => {
                                              let timer_tx = timer_tx.clone();
                                              let timer_id = id.clone();
                                              let handle = tokio::spawn(async move {
                                                  tokio::time::sleep(duration).await;
                                                  let _ = timer_tx.send(ServiceEvent::TimerExpired(timer_id)).await;
                                              });
                                              timers.insert(id, handle.abort_handle());
                                          }
                                          GameCommand::CancelTimer { id, .. } => {
                                              if let Some(handle) = timers.remove(&id) {
                                                  handle.abort();
                                              }
                                          }
                                          GameCommand::CreateInterval { id } => {
                                               let timer_tx = timer_tx.clone();
                                               let interval_id = id.clone();
                                               let handle = tokio::spawn(async move {
                                                  loop {
                                                       tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                                       if timer_tx.send(ServiceEvent::Interval(interval_id.clone())).await.is_err() {
                                                           break;
                                                       }
                                                  }
                                              });
                                              timers.insert(id, handle.abort_handle());
                                          }
                                          GameCommand::CancelInterval { id } => {
                                               if let Some(handle) = timers.remove(&id) {
                                                  handle.abort();
                                              }
                                          }
                                      }
                                  }

                                  // Broadcast Game Views
                                  if !update.views.is_empty() {
                                      for (view, view_update) in update.views {
                                          match view {
                                              openplay_basic::room::RoomView::Position(pos) => {
                                                  if let Some(player_state) = room.state.players.get(&pos) {
                                                      connection_handle
                                                          .send_game_view_update(
                                                              view_update,
                                                              player_state.player.id.clone(),
                                                          )
                                                          .await?;
                                                  }
                                              }
                                              openplay_basic::room::RoomView::Neutral => {
                                                  let mut all_users = Vec::new();
                                                  for p in room.state.players.values() {
                                                      all_users.push(p.player.id.clone());
                                                  }
                                                  for o in room.state.observers.keys() {
                                                      all_users.push(o.clone());
                                                  }
                                                  
                                                  for user_id in all_users {
                                                       connection_handle
                                                          .send_game_view_update(
                                                              view_update.clone(),
                                                              user_id,
                                                          )
                                                          .await?;
                                                  }
                                              }
                                              _ => {}
                                          }
                                      }
                                  }
                             }
                        }

                        continue;
                    }
                }

                let sequenced_update = match evt {
                    ServiceEvent::Action(action) => {
                        match action {
                            Action {
                                data: openplay_basic::user::ActionData::GameAction(game_action),
                                ..
                            } => {
                                // Extract seq from GameActionData
                                let seq = game_action.ref_version;
                                let game_event =
                                    GameEvent::Action(Action {
                                        source: action.source, // Reconstruct action
                                        data: openplay_basic::user::ActionData::GameAction(game_action),
                                    });
                                SequencedGameUpdate { event: game_event, seq }
                            }
                             _ => continue, // Already handled RoomAction above
                        }
                    }
                    ServiceEvent::TimerExpired(id) => SequencedGameUpdate {
                        event: GameEvent::TimerExpired(TimeExpired {
                            timer_id: id.clone(),
                        }),
                        seq: 0, // System events might not need seq check or use latest
                    },
                    ServiceEvent::Interval(id) => SequencedGameUpdate {
                        event: GameEvent::Interval(Interval {
                            interval_id: id.clone(),
                        }),
                        seq: 0,
                    },
                };

                // Process Game Event
                let room_context = openplay_basic::room::RoomContext {};
                let update = game.handle_action(&room_context, sequenced_update);

                // Handle Game Commands (Timer/Interval)
                for command in update.commands {
                    match command {
                        GameCommand::CreateTimer { id, duration } => {
                            let timer_tx = timer_tx.clone();
                            let timer_id = id.clone();
                            let handle = tokio::spawn(async move {
                                tokio::time::sleep(duration).await;
                                let _ = timer_tx.send(ServiceEvent::TimerExpired(timer_id)).await;
                            });
                            timers.insert(id, handle.abort_handle());
                        }
                        GameCommand::CancelTimer { id, .. } => {
                            if let Some(handle) = timers.remove(&id) {
                                handle.abort();
                            }
                        }
                        GameCommand::CreateInterval { id } => {
                            // TODO: Implement Interval logic if needed, similar to Timer but loop
                             let timer_tx = timer_tx.clone();
                            let interval_id = id.clone();
                             let handle = tokio::spawn(async move {
                                loop {
                                     // Hardcoded interval duration or pass it in command?
                                     // Assuming some default or passed duration. 
                                     // For now, let's just sleep 1s as placeholder if not specified
                                     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                     if timer_tx.send(ServiceEvent::Interval(interval_id.clone())).await.is_err() {
                                         break;
                                     }
                                }
                            });
                            timers.insert(id, handle.abort_handle());
                        }
                        GameCommand::CancelInterval { id } => {
                             if let Some(handle) = timers.remove(&id) {
                                handle.abort();
                            }
                        }
                    }
                }

                // Broadcast Game View Updates
                if !update.views.is_empty() {
                    let mut recipients_handled = std::collections::HashSet::new();

                    // 1. First Pass: Handle Position-specific views (most specific)
                    for (view, view_update) in &update.views {
                         if let openplay_basic::room::RoomView::Position(pos) = view {
                             if let Some(player_state) = room.state.players.get(pos) {
                                 let pid = player_state.player.id.clone();
                                 connection_handle
                                     .send_game_view_update(
                                         view_update.clone(),
                                         pid.clone(),
                                     )
                                     .await?;
                                 recipients_handled.insert(pid);
                             }
                         }
                    }

                    // 2. Second Pass: Handle Neutral views (broadcasting to remaining)
                    if let Some(neutral_update) = update.views.get(&openplay_basic::room::RoomView::Neutral) {
                         let mut all_users = Vec::new();
                         for p in room.state.players.values() {
                             if !recipients_handled.contains(&p.player.id) {
                                 all_users.push(p.player.id.clone());
                             }
                         }
                         for o in room.state.observers.keys() {
                             // Observers don't have position views usually
                             if !recipients_handled.contains(o) {
                                 all_users.push(o.clone());
                             }
                         }
                         
                         for user_id in all_users {
                              connection_handle
                                 .send_game_view_update(
                                     neutral_update.clone(),
                                     user_id,
                                 )
                                 .await?;
                         }
                    }
                }
                
                // Broadcast Snapshot/State update if needed
                // Only if state changed significantly? Game logic determines via GameUpdate
                 connection_handle
                    .broadcast_room_update(RoomUpdate {
                        room: room.clone(),
                        events: vec![], // TODO: Add game events if mapped to RoomEvent?
                    })
                    .await?;

            }
            Ok(())
        };

        let join_handle = tokio::spawn(task);

        RoomServiceHandle {
            cancel_token: handle_ct,
            join_handle,
        }
    }

    async fn handle_room_action(
        room: &mut Room,
        connection_handle: &ConnectionHandle,
        source_id: Option<&UserId>,
        action: RoomActionData,
    ) -> Result<(), RoomServiceError> {
        let user_id = match source_id {
            Some(id) => id,
            None => return Ok(()), // System actions on room?
        };

        match action {
            RoomActionData::Chat(chat) => {
                connection_handle
                    .broadcast_room_update(RoomUpdate {
                        room: room.clone(),
                        events: vec![RoomEvent::UserChat(UserActionEvent {
                            user_id: user_id.clone(),
                            data: chat,
                        })],
                    })
                    .await?;
            }
            RoomActionData::ChangeReadyState(change) => {
                if let Some(player_state) = room
                    .state
                    .players
                    .values_mut()
                    .find(|state| state.player.id == *user_id)
                {
                    player_state.id_ready = change.is_ready;
                    connection_handle
                        .broadcast_room_update(RoomUpdate {
                            room: room.clone(),
                            events: vec![RoomEvent::UserReady(UserActionEvent {
                                user_id: user_id.clone(),
                                data: change,
                            })],
                        })
                        .await?;
                }
            }
            RoomActionData::PositionChange(change) => {
                // Logic for sitting down / changing seats
                // This is a simplified version; real logic needs validation (is seat empty?)
                
                // 1. If user is observer and wants to sit
                // ... (Implement logic to move from observer to player and assign position)
                // 2. If user is player and wants to move/stand up
                // ... (Implement other cases as needed)
            }
            RoomActionData::Leave => {
                if let Some(_) = room.remove_player(user_id) {
                     connection_handle.broadcast_room_update(RoomUpdate {
                         room: room.clone(),
                         events: vec![RoomEvent::UserLeave(UserActionEvent {
                             user_id: user_id.clone(),
                             data: ()
                         })]
                     }).await?;
                }
            }
            RoomActionData::RoomManage(manage) => {
                // Check if user is owner
                if room.info.owner == *user_id {
                    match manage {
                        RoomManage::KickOut(kick) => {
                             if let Some(_) = room.remove_player(&kick.player) {
                                 connection_handle.broadcast_room_update(RoomUpdate {
                                     room: room.clone(),
                                     events: vec![RoomEvent::UserKickedOut(UserActionEvent {
                                         user_id: kick.player,
                                         data: ()
                                     })]
                                 }).await?;
                             }
                        }
                        _ => {}
                    }
                }
            }
            RoomActionData::Reconnect => {
                 connection_handle.broadcast_room_update(RoomUpdate {
                     room: room.clone(),
                     events: vec![RoomEvent::UserReconnected(UserActionEvent {
                         user_id: user_id.clone(),
                         data: ()
                     })]
                 }).await?;
            }
        }
        Ok(())
    }
}
