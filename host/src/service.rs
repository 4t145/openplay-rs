use std::collections::HashMap;
use std::sync::Arc;

use openplay_basic::{
    game::{
        DynGame, GameCommand, GameEvent, GameUpdate, GameViewUpdate, Interval, SequencedGameUpdate,
        TimeExpired,
    },
    room::{
        Room, RoomEvent, RoomObserverState, RoomObserverView, RoomPhase, RoomPhaseKind,
        RoomPlayerState, RoomUpdate, RoomUserPosition, UserActionEvent,
    },
    user::{
        Action, DynUserAgent, User, UserId,
        room_action::{RoomActionData, RoomManage},
    },
};
use tokio::task::AbortHandle;
use tokio_util::sync::CancellationToken;

use crate::connection::{ConnectionController, ConnectionHandle};

/// Factory for creating bot user agents. Implement this trait in the application
/// layer to provide game-specific bot logic without coupling the host library
/// to any particular game.
pub trait BotFactory: Send + Sync + 'static {
    /// Create a bot user agent and its User identity.
    /// `name` is an optional display name for the bot.
    fn create_bot(&self, name: Option<String>) -> (User, DynUserAgent);
}

pub struct RoomService {
    pub game: DynGame,
    pub room: Room,
    pub user_agents: HashMap<UserId, DynUserAgent>,
    pub bot_factory: Option<Arc<dyn BotFactory>>,
}

#[derive(Debug, thiserror::Error)]
pub enum RoomServiceError {
    #[error("Connection handle error")]
    ConnectionHandleError(#[from] crate::connection::ConnectionHandleError),
}

pub struct RoomServiceHandle {
    pub cancel_token: CancellationToken,
    pub join_handle: tokio::task::JoinHandle<Result<(), RoomServiceError>>,
    pub connection_controller: ConnectionController,
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
            user_agents: HashMap::new(),
            bot_factory: None,
        }
    }
    pub fn new<G: openplay_basic::game::Game>(game: G, room: Room) -> Self {
        Self::from_dyn(Box::new(game), room)
    }

    pub fn with_bot_factory(mut self, factory: Arc<dyn BotFactory>) -> Self {
        self.bot_factory = Some(factory);
        self
    }

    pub fn run(self) -> RoomServiceHandle {
        let Self {
            mut game,
            mut room,
            user_agents,
            bot_factory,
        } = self;

        let (action_tx, mut action_rx) = tokio::sync::mpsc::channel::<Action>(32);
        let (timer_tx, mut timer_rx) = tokio::sync::mpsc::channel::<ServiceEvent>(32);

        // Create the connection handle and controller
        let (connection_handle, connection_controller) =
            ConnectionHandle::run(user_agents, action_tx.clone());

        // Forward user actions to the main event loop
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

        // Clone controller for use inside the task
        let task_controller = connection_controller.clone();

        let task = async move {
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

                // Filter non-game events first (Room Actions)
                if let ServiceEvent::Action(action) = &evt {
                    if let openplay_basic::user::ActionData::RoomAction(room_action) = &action.data
                    {
                        let start_update = Self::handle_room_action(
                            &mut room,
                            &mut game,
                            &task_controller,
                            action.source(),
                            room_action.clone(),
                            bot_factory.as_ref(),
                        )
                        .await?;

                        // If StartGame returned a GameUpdate, process it inline
                        if let Some(update) = start_update {
                            Self::process_game_commands(
                                update.commands,
                                &mut room,
                                &mut timers,
                                &timer_tx,
                            );

                            Self::broadcast_game_views(&update.views, &room, &task_controller)
                                .await?;

                            // Broadcast room state update
                            task_controller
                                .broadcast_room_update(RoomUpdate {
                                    room: room.clone(),
                                    events: vec![],
                                })
                                .await?;
                        }

                        continue;
                    }
                }

                let sequenced_update = match evt {
                    ServiceEvent::Action(action) => {
                        // Phase Guard: reject GameActions during Waiting phase
                        if matches!(room.state.phase.kind, RoomPhaseKind::Waiting) {
                            if matches!(
                                action.data,
                                openplay_basic::user::ActionData::GameAction(_)
                            ) {
                                tracing::warn!(
                                    "Phase Guard: GameAction rejected during Waiting phase (user {:?})",
                                    action.source()
                                );
                                continue;
                            }
                        }

                        match action {
                            Action {
                                data: openplay_basic::user::ActionData::GameAction(game_action),
                                ..
                            } => {
                                // Extract seq from GameActionData
                                let seq = game_action.ref_version;
                                let game_event = GameEvent::Action(Action {
                                    source: action.source, // Reconstruct action
                                    data: openplay_basic::user::ActionData::GameAction(game_action),
                                });
                                SequencedGameUpdate {
                                    event: game_event,
                                    seq,
                                }
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
                let room_context = openplay_basic::room::RoomContext::new(room.clone());
                let update = game.handle_action(&room_context, sequenced_update);

                Self::process_game_commands(update.commands, &mut room, &mut timers, &timer_tx);

                Self::broadcast_game_views(&update.views, &room, &task_controller).await?;

                // Broadcast room state update
                task_controller
                    .broadcast_room_update(RoomUpdate {
                        room: room.clone(),
                        events: vec![],
                    })
                    .await?;
            }

            // Clean up connection handle when task finishes
            connection_handle.quit().await;
            Ok(())
        };

        let join_handle = tokio::spawn(task);

        RoomServiceHandle {
            cancel_token: handle_ct,
            join_handle,
            connection_controller,
        }
    }

    async fn handle_room_action(
        room: &mut Room,
        game: &mut DynGame,
        connection_controller: &ConnectionController,
        source_id: Option<&UserId>,
        action: RoomActionData,
        bot_factory: Option<&Arc<dyn BotFactory>>,
    ) -> Result<Option<GameUpdate>, RoomServiceError> {
        let user_id = match source_id {
            Some(id) => id,
            None => return Ok(None), // System actions on room?
        };

        let is_gaming = matches!(room.state.phase.kind, RoomPhaseKind::Gaming);

        // Phase Guard: reject destructive room actions during Gaming phase
        if is_gaming {
            match &action {
                RoomActionData::PositionChange(_) | RoomActionData::ChangeReadyState(_) => {
                    tracing::warn!(
                        "Phase Guard: {:?} rejected during Gaming phase (user {})",
                        std::mem::discriminant(&action),
                        user_id
                    );
                    return Ok(None);
                }
                RoomActionData::RoomManage(manage) => {
                    match manage {
                        RoomManage::StartGame | RoomManage::AddBot(_) | RoomManage::KickOut(_) => {
                            tracing::warn!(
                                "Phase Guard: RoomManage action rejected during Gaming phase (user {})",
                                user_id
                            );
                            return Ok(None);
                        }
                        // SetGameConfig and SetRoomConfig are allowed during Gaming
                        _ => {}
                    }
                }
                // Join, Leave, Chat, Reconnect are allowed during Gaming
                _ => {}
            }
        }

        match action {
            RoomActionData::Join(join) => {
                // Check if user is already in room (as player or observer)
                let in_players = room.state.players.values().any(|p| p.player.id == *user_id);
                let in_observers = room.state.observers.contains_key(user_id);

                if !in_players && !in_observers {
                    // New user: add to observers
                    let user = User {
                        nickname: join.nickname,
                        id: user_id.clone(),
                        avatar_url: None,
                        is_bot: false,
                    };
                    room.state.observers.insert(
                        user_id.clone(),
                        RoomObserverState {
                            is_connected: true,
                            view: RoomObserverView::default(),
                            player: user,
                        },
                    );
                    tracing::info!("User {} joined room as observer", user_id);
                    connection_controller
                        .broadcast_room_update(RoomUpdate {
                            room: room.clone(),
                            events: vec![RoomEvent::UserJoin(UserActionEvent {
                                user_id: user_id.clone(),
                                data: (),
                            })],
                        })
                        .await?;
                } else {
                    // User already in room (reconnect scenario): mark as connected and push current state
                    tracing::info!("User {} rejoined room (already present)", user_id);
                    // Mark user as connected
                    if let Some(ps) = room
                        .state
                        .players
                        .values_mut()
                        .find(|ps| ps.player.id == *user_id)
                    {
                        ps.is_connected = true;
                    } else if let Some(os) = room.state.observers.get_mut(user_id) {
                        os.is_connected = true;
                    }
                    connection_controller
                        .broadcast_room_update(RoomUpdate {
                            room: room.clone(),
                            events: vec![RoomEvent::UserReconnected(UserActionEvent {
                                user_id: user_id.clone(),
                                data: (),
                            })],
                        })
                        .await?;
                    // During Gaming phase, also push current game view to the reconnecting user
                    Self::send_current_game_view_to_user(
                        game,
                        room,
                        user_id,
                        connection_controller,
                    )
                    .await?;
                }
            }
            RoomActionData::Chat(chat) => {
                connection_controller
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
                    connection_controller
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
                // Validate the user matches the 'from' position
                let valid = match &change.from {
                    RoomUserPosition::Observer(_) => room.state.observers.contains_key(user_id),
                    RoomUserPosition::Player(pos) => room
                        .state
                        .players
                        .get(pos)
                        .map_or(false, |p| p.player.id == *user_id),
                };

                if !valid {
                    tracing::warn!(
                        "PositionChange: user {:?} not at claimed 'from' position {:?}",
                        user_id,
                        change.from
                    );
                    return Ok(None);
                }

                match (&change.from, &change.to) {
                    // Observer -> Player: sit down at a seat
                    (RoomUserPosition::Observer(_), RoomUserPosition::Player(target_pos)) => {
                        // Check target seat is empty
                        if room.state.players.contains_key(target_pos) {
                            tracing::warn!(
                                "PositionChange: target seat {:?} is occupied",
                                target_pos
                            );
                            return Ok(None);
                        }
                        // Remove from observers
                        if let Some(obs) = room.state.observers.remove(user_id) {
                            // Insert as player
                            room.state.players.insert(
                                target_pos.clone(),
                                RoomPlayerState {
                                    id_ready: false,
                                    is_connected: obs.is_connected,
                                    player: obs.player,
                                },
                            );
                        }
                    }
                    // Player -> Observer: stand up from a seat
                    (RoomUserPosition::Player(from_pos), RoomUserPosition::Observer(view)) => {
                        if let Some(player_state) = room.state.players.remove(from_pos) {
                            room.state.observers.insert(
                                user_id.clone(),
                                RoomObserverState {
                                    is_connected: player_state.is_connected,
                                    view: view.clone(),
                                    player: player_state.player,
                                },
                            );
                        }
                    }
                    // Player -> Player: swap seats
                    (RoomUserPosition::Player(from_pos), RoomUserPosition::Player(target_pos)) => {
                        // Check target seat is empty
                        if room.state.players.contains_key(target_pos) {
                            tracing::warn!(
                                "PositionChange: target seat {:?} is occupied",
                                target_pos
                            );
                            return Ok(None);
                        }
                        if let Some(player_state) = room.state.players.remove(from_pos) {
                            room.state.players.insert(target_pos.clone(), player_state);
                        }
                    }
                    // Observer -> Observer: not meaningful, ignore
                    _ => {
                        return Ok(None);
                    }
                }

                // Broadcast the position change
                connection_controller
                    .broadcast_room_update(RoomUpdate {
                        room: room.clone(),
                        events: vec![RoomEvent::UserChangePosition(UserActionEvent {
                            user_id: user_id.clone(),
                            data: change,
                        })],
                    })
                    .await?;
            }
            RoomActionData::Leave => {
                if is_gaming {
                    // During Gaming: mark player as disconnected instead of removing
                    let marked = if let Some(ps) = room
                        .state
                        .players
                        .values_mut()
                        .find(|ps| ps.player.id == *user_id)
                    {
                        ps.is_connected = false;
                        true
                    } else if room.state.observers.contains_key(user_id) {
                        // Observer can leave normally even during gaming
                        false
                    } else {
                        false
                    };
                    if marked {
                        connection_controller
                            .broadcast_room_update(RoomUpdate {
                                room: room.clone(),
                                events: vec![RoomEvent::UserLeave(UserActionEvent {
                                    user_id: user_id.clone(),
                                    data: (),
                                })],
                            })
                            .await?;
                    } else {
                        // Observer leaving during gaming — remove normally
                        if let Some(_) = room.state.observers.remove(user_id) {
                            connection_controller
                                .broadcast_room_update(RoomUpdate {
                                    room: room.clone(),
                                    events: vec![RoomEvent::UserLeave(UserActionEvent {
                                        user_id: user_id.clone(),
                                        data: (),
                                    })],
                                })
                                .await?;
                        }
                    }
                } else {
                    // During Waiting: remove player normally
                    if let Some(_) = room.remove_player(user_id) {
                        connection_controller
                            .broadcast_room_update(RoomUpdate {
                                room: room.clone(),
                                events: vec![RoomEvent::UserLeave(UserActionEvent {
                                    user_id: user_id.clone(),
                                    data: (),
                                })],
                            })
                            .await?;
                    }
                }
            }
            RoomActionData::RoomManage(manage) => {
                // Check if user is owner
                if room.info.owner == *user_id {
                    match manage {
                        RoomManage::KickOut(kick) => {
                            if let Some(_) = room.remove_player(&kick.player) {
                                connection_controller
                                    .broadcast_room_update(RoomUpdate {
                                        room: room.clone(),
                                        events: vec![RoomEvent::UserKickedOut(UserActionEvent {
                                            user_id: kick.player,
                                            data: (),
                                        })],
                                    })
                                    .await?;
                            }
                        }
                        RoomManage::SetGameConfig(config) => {
                            room.info.game_config = Some(config.clone());
                            if let Err(e) = game.apply_config(config.clone()) {
                                tracing::warn!("Failed to apply game config: {}", e);
                            } else {
                                connection_controller
                                    .broadcast_room_update(RoomUpdate {
                                        room: room.clone(),
                                        events: vec![RoomEvent::RoomManage(UserActionEvent {
                                            user_id: user_id.clone(),
                                            data: RoomManage::SetGameConfig(config),
                                        })],
                                    })
                                    .await?;
                            }
                        }
                        RoomManage::AddBot(add_bot) => {
                            // Check seat is empty
                            if room.state.players.contains_key(&add_bot.position) {
                                tracing::warn!("AddBot: seat {:?} is occupied", add_bot.position);
                                return Ok(None);
                            }

                            let Some(factory) = bot_factory else {
                                tracing::warn!("AddBot: no bot factory configured");
                                return Ok(None);
                            };

                            // Create bot via factory
                            let (bot_user, dyn_agent) = factory.create_bot(add_bot.name.clone());
                            let bot_id = bot_user.id.clone();

                            // Sit the bot at the requested position (bots are auto-ready)
                            room.state.players.insert(
                                add_bot.position.clone(),
                                RoomPlayerState {
                                    id_ready: true,
                                    is_connected: true,
                                    player: bot_user,
                                },
                            );

                            // Connect the bot agent to the connection controller
                            if let Err(e) = connection_controller
                                .user_connect(bot_id.clone(), dyn_agent)
                                .await
                            {
                                tracing::error!("AddBot: failed to connect bot agent: {}", e);
                                // Rollback: remove from room
                                room.state.players.remove(&add_bot.position);
                                return Ok(None);
                            }

                            // Broadcast updated room state
                            connection_controller
                                .broadcast_room_update(RoomUpdate {
                                    room: room.clone(),
                                    events: vec![RoomEvent::UserJoin(UserActionEvent {
                                        user_id: bot_id,
                                        data: (),
                                    })],
                                })
                                .await?;
                        }
                        RoomManage::SetRoomConfig => {
                            // Not implemented yet
                        }
                        RoomManage::StartGame => {
                            // Validate: enough players and all ready
                            if room.state.player_count() < 3 {
                                tracing::warn!(
                                    "StartGame: not enough players ({}/3)",
                                    room.state.player_count()
                                );
                                return Ok(None);
                            }
                            if !room.state.players.values().all(|p| p.id_ready) {
                                tracing::warn!("StartGame: not all players are ready");
                                return Ok(None);
                            }

                            // Trigger game start and return the update for the caller to process
                            let game_update = game.handle_action(
                                &openplay_basic::room::RoomContext::new(room.clone()),
                                SequencedGameUpdate {
                                    event: GameEvent::GameStart,
                                    seq: 0,
                                },
                            );

                            // Transition room phase to Gaming
                            room.state.phase = RoomPhase {
                                kind: RoomPhaseKind::Gaming,
                                since: chrono::Utc::now(),
                            };

                            return Ok(Some(game_update));
                        }
                    }
                } else {
                    tracing::warn!(
                        "RoomManage: user {:?} is not room owner ({:?}), action {:?} rejected",
                        user_id,
                        room.info.owner,
                        manage
                    );
                }
            }
            RoomActionData::Reconnect => {
                // Mark user as connected
                if let Some(ps) = room
                    .state
                    .players
                    .values_mut()
                    .find(|ps| ps.player.id == *user_id)
                {
                    ps.is_connected = true;
                } else if let Some(os) = room.state.observers.get_mut(user_id) {
                    os.is_connected = true;
                }
                connection_controller
                    .broadcast_room_update(RoomUpdate {
                        room: room.clone(),
                        events: vec![RoomEvent::UserReconnected(UserActionEvent {
                            user_id: user_id.clone(),
                            data: (),
                        })],
                    })
                    .await?;
                // During Gaming phase, also push current game view to the reconnecting user
                Self::send_current_game_view_to_user(game, room, user_id, connection_controller)
                    .await?;
            }
        }
        Ok(None)
    }

    /// Process game commands from a GameUpdate: create/cancel timers and intervals,
    /// handle GameOver transition.
    fn process_game_commands(
        commands: Vec<GameCommand>,
        room: &mut Room,
        timers: &mut HashMap<openplay_basic::game::Id, AbortHandle>,
        timer_tx: &tokio::sync::mpsc::Sender<ServiceEvent>,
    ) {
        for command in commands {
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
                            if timer_tx
                                .send(ServiceEvent::Interval(interval_id.clone()))
                                .await
                                .is_err()
                            {
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
                GameCommand::GameOver => {
                    // Transition room back to waiting state
                    room.state.phase = RoomPhase {
                        kind: RoomPhaseKind::Waiting,
                        since: chrono::Utc::now(),
                    };
                    // Reset ready state for human players only;
                    // bots stay ready so the owner can immediately start the next game.
                    for player in room.state.players.values_mut() {
                        if !player.player.is_bot {
                            player.id_ready = false;
                        }
                    }
                }
            }
        }
    }

    /// Broadcast game view updates to all connected users:
    /// - Position-specific views go to the player at that seat.
    /// - Neutral view goes to remaining players and all observers.
    async fn broadcast_game_views(
        views: &HashMap<openplay_basic::room::RoomView, GameViewUpdate>,
        room: &Room,
        controller: &ConnectionController,
    ) -> Result<(), RoomServiceError> {
        if views.is_empty() {
            return Ok(());
        }

        let mut recipients_handled = std::collections::HashSet::new();

        // 1. Position-specific views
        for (view, view_update) in views {
            if let openplay_basic::room::RoomView::Position(pos) = view {
                if let Some(player_state) = room.state.players.get(pos) {
                    let pid = player_state.player.id.clone();
                    controller
                        .send_game_view_update(view_update.clone(), pid.clone())
                        .await?;
                    recipients_handled.insert(pid);
                }
            }
        }

        // 2. Neutral view to remaining users
        if let Some(neutral_update) = views.get(&openplay_basic::room::RoomView::Neutral) {
            let mut remaining_users = Vec::new();
            for p in room.state.players.values() {
                if !recipients_handled.contains(&p.player.id) {
                    remaining_users.push(p.player.id.clone());
                }
            }
            for o in room.state.observers.keys() {
                if !recipients_handled.contains(o) {
                    remaining_users.push(o.clone());
                }
            }
            for user_id in remaining_users {
                controller
                    .send_game_view_update(neutral_update.clone(), user_id)
                    .await?;
            }
        }

        Ok(())
    }

    /// Send the current game view to a specific user (for reconnect/join during Gaming).
    /// Looks up the user's seat position and sends the appropriate view.
    /// If the user is an observer, sends the neutral view.
    async fn send_current_game_view_to_user(
        game: &DynGame,
        room: &Room,
        user_id: &UserId,
        controller: &ConnectionController,
    ) -> Result<(), RoomServiceError> {
        if !matches!(room.state.phase.kind, RoomPhaseKind::Gaming) {
            return Ok(());
        }

        let room_context = openplay_basic::room::RoomContext::new(room.clone());
        let Some(update) = game.current_view(&room_context) else {
            return Ok(());
        };

        // Find user's seat position
        let user_position = room.state.players.iter().find_map(|(pos, ps)| {
            if ps.player.id == *user_id {
                Some(pos.clone())
            } else {
                None
            }
        });

        if let Some(pos) = user_position {
            // Player: send position-specific view
            let view_key = openplay_basic::room::RoomView::Position(pos);
            if let Some(view_update) = update.views.get(&view_key) {
                controller
                    .send_game_view_update(view_update.clone(), user_id.clone())
                    .await?;
                return Ok(());
            }
        }

        // Observer or no position view found: send neutral view
        if let Some(neutral_update) = update.views.get(&openplay_basic::room::RoomView::Neutral) {
            controller
                .send_game_view_update(neutral_update.clone(), user_id.clone())
                .await?;
        }

        Ok(())
    }
}
