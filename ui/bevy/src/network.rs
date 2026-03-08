use std::sync::{Arc, Mutex, mpsc};

use bevy::prelude::*;
use futures::StreamExt;
use openplay_basic::{
    room::{Room, RoomPhaseKind, RoomUpdate, Update},
    user::{
        ActionData,
        room_action::{JoinRoom, RoomActionData},
    },
};
use openplay_client::{RoomClient, SseEvent, authenticate, default_user_dir, load_first_or_create_random};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::{
    data::CurrentUserInfo,
    games::{ActiveGame, GameKind, GameViewSnapshot, LatestGameView},
    state::MainState,
    ui::general_ui::GeneralUiPlayerUsername,
};

const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:3000";
const DEFAULT_ROOM_PATH: &str = "/room/ua";
const RECONNECT_INTERVAL_SECS: f32 = 2.0;
const RECONNECT_MAX_ATTEMPTS: u32 = 10;

pub struct NetworkPlugin;

#[derive(Resource, Debug, Clone)]
pub struct ConnectTarget {
    pub server_url: String,
    pub room_path: String,
}

impl Default for ConnectTarget {
    fn default() -> Self {
        Self {
            server_url: DEFAULT_SERVER_URL.to_string(),
            room_path: DEFAULT_ROOM_PATH.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum NetworkCommand {
    SendAction(ActionData),
    Disconnect,
}

#[derive(Debug)]
enum RuntimeEvent {
    IdentityLoaded(openplay_basic::user::User),
    Connected,
    Update(Update),
    ConnectFailed(String),
    Disconnected,
}

#[derive(Resource, Default)]
pub struct ConnectionRuntime {
    pub command_tx: Option<UnboundedSender<NetworkCommand>>,
    event_rx: Option<Arc<Mutex<mpsc::Receiver<RuntimeEvent>>>>,
}

impl ConnectionRuntime {
    pub fn send_action(&self, action: ActionData) {
        if let Some(command_tx) = self.command_tx.as_ref() {
            let _ = command_tx.send(NetworkCommand::SendAction(action));
        }
    }

    fn disconnect(&mut self) {
        if let Some(command_tx) = self.command_tx.take() {
            let _ = command_tx.send(NetworkCommand::Disconnect);
        }
    }

    pub fn request_disconnect(&self) {
        if let Some(command_tx) = self.command_tx.as_ref() {
            let _ = command_tx.send(NetworkCommand::Disconnect);
        }
    }

    fn replace_handles(
        &mut self,
        command_tx: UnboundedSender<NetworkCommand>,
        event_rx: Arc<Mutex<mpsc::Receiver<RuntimeEvent>>>,
    ) {
        self.disconnect();
        self.command_tx = Some(command_tx);
        self.event_rx = Some(event_rx);
    }
}

#[derive(Resource, Default, Clone)]
pub struct RoomSnapshot {
    pub room: Option<Room>,
    pub messages: Vec<String>,
}

#[derive(Resource)]
pub struct ReconnectState {
    pub attempts: u32,
    pub in_flight: bool,
    pub timer: Timer,
    pub last_error: Option<String>,
}

impl Default for ReconnectState {
    fn default() -> Self {
        Self {
            attempts: 0,
            in_flight: false,
            timer: Timer::from_seconds(RECONNECT_INTERVAL_SECS, TimerMode::Once),
            last_error: None,
        }
    }
}

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConnectTarget>()
            .init_resource::<ConnectionRuntime>()
            .init_resource::<RoomSnapshot>()
            .init_resource::<ReconnectState>()
            .add_systems(OnEnter(MainState::ConnectingGameRoom), start_initial_connection)
            .add_systems(OnEnter(MainState::ReconnectingGameRoom), enter_reconnecting_state)
            .add_systems(
                Update,
                (
                    poll_runtime_events,
                    drive_reconnect_attempts,
                    refresh_top_bar_username,
                    complete_game_loading,
                ),
            );
    }
}

fn start_initial_connection(
    mut runtime: ResMut<ConnectionRuntime>,
    mut reconnect: ResMut<ReconnectState>,
    connect_target: Res<ConnectTarget>,
    mut room_snapshot: ResMut<RoomSnapshot>,
    mut game_view_snapshot: ResMut<GameViewSnapshot>,
    mut active_game: ResMut<ActiveGame>,
) {
    room_snapshot.room = None;
    room_snapshot.messages.clear();
    game_view_snapshot.latest = None;
    active_game.kind = None;
    reconnect.attempts = 0;
    reconnect.in_flight = true;
    reconnect.last_error = None;

    let (command_tx, event_rx) = spawn_connection_runtime(connect_target.server_url.clone(), connect_target.room_path.clone());
    runtime.replace_handles(command_tx, event_rx);
}

fn enter_reconnecting_state(
    mut runtime: ResMut<ConnectionRuntime>,
    mut reconnect: ResMut<ReconnectState>,
    mut game_view_snapshot: ResMut<GameViewSnapshot>,
    mut active_game: ResMut<ActiveGame>,
) {
    runtime.disconnect();
    game_view_snapshot.latest = None;
    active_game.kind = None;
    reconnect.attempts = 0;
    reconnect.in_flight = false;
    reconnect.last_error = None;
    reconnect.timer = Timer::from_seconds(0.0, TimerMode::Once);
}

fn drive_reconnect_attempts(
    time: Res<Time>,
    state: Res<State<MainState>>,
    connect_target: Res<ConnectTarget>,
    mut reconnect: ResMut<ReconnectState>,
    mut runtime: ResMut<ConnectionRuntime>,
    mut next_state: ResMut<NextState<MainState>>,
) {
    if *state.get() != MainState::ReconnectingGameRoom || reconnect.in_flight {
        return;
    }

    reconnect.timer.tick(time.delta());
    if !reconnect.timer.is_finished() {
        return;
    }

    if reconnect.attempts >= RECONNECT_MAX_ATTEMPTS {
        next_state.set(MainState::Lobby);
        return;
    }

    reconnect.in_flight = true;
    reconnect.attempts += 1;
    let (command_tx, event_rx) = spawn_connection_runtime(connect_target.server_url.clone(), connect_target.room_path.clone());
    runtime.replace_handles(command_tx, event_rx);
}

fn refresh_top_bar_username(
    current_user_info: Res<CurrentUserInfo>,
    mut text_query: Query<&mut Text, With<GeneralUiPlayerUsername>>,
) {
    if !current_user_info.is_changed() {
        return;
    }

    let username = current_user_info
        .user
        .as_ref()
        .map(|user| user.nickname.clone())
        .unwrap_or_else(|| "<anon>".to_string());

    for mut text in text_query.iter_mut() {
        text.0 = username.clone();
    }
}

fn poll_runtime_events(
    state: Res<State<MainState>>,
    mut runtime: ResMut<ConnectionRuntime>,
    mut reconnect: ResMut<ReconnectState>,
    mut room_snapshot: ResMut<RoomSnapshot>,
    mut game_view_snapshot: ResMut<GameViewSnapshot>,
    mut active_game: ResMut<ActiveGame>,
    mut current_user_info: ResMut<CurrentUserInfo>,
    mut next_state: ResMut<NextState<MainState>>,
) {
    let Some(event_rx) = runtime.event_rx.as_ref() else {
        return;
    };

    let Ok(event_rx) = event_rx.lock() else {
        return;
    };

    let mut pending_events = Vec::new();
    while let Ok(event) = event_rx.try_recv() {
        pending_events.push(event);
    }
    drop(event_rx);

    for event in pending_events {
        match event {
            RuntimeEvent::IdentityLoaded(user) => {
                current_user_info.user = Some(user);
            }
            RuntimeEvent::Connected => {
                reconnect.in_flight = false;
                reconnect.last_error = None;
                game_view_snapshot.latest = None;
                active_game.kind = None;
                next_state.set(MainState::GameRoom);
            }
            RuntimeEvent::Update(update) => match update {
                Update::Room(room_update) => {
                    update_room_snapshot(&mut room_snapshot, *room_update);
                }
                Update::GameView(game_view) => {
                    game_view_snapshot.latest = Some(LatestGameView {
                        version: game_view.new_view.version,
                        data: game_view.new_view.data.clone(),
                    });

                    if let Some(room) = room_snapshot.room.as_ref() {
                        let app_id = room.info.game_meta.app.id.as_str();
                        active_game.kind = GameKind::from_app_id(app_id);

                        let should_enter_game = matches!(room.state.phase.kind, RoomPhaseKind::Gaming);
                        if *state.get() == MainState::GameRoom && should_enter_game {
                            next_state.set(MainState::GameLoading);
                        }
                    }
                }
            },
            RuntimeEvent::ConnectFailed(error) => {
                reconnect.in_flight = false;
                reconnect.last_error = Some(error.clone());
                let is_reconnecting = *state.get() == MainState::ReconnectingGameRoom;
                if is_reconnecting {
                    reconnect.timer = Timer::from_seconds(RECONNECT_INTERVAL_SECS, TimerMode::Once);
                } else {
                    next_state.set(MainState::Lobby);
                }
            }
            RuntimeEvent::Disconnected => {
                runtime.command_tx = None;
                reconnect.in_flight = false;
                match *state.get() {
                    MainState::ConnectingGameRoom => {
                        next_state.set(MainState::Lobby);
                    }
                    MainState::GameRoom => {
                        reconnect.timer = Timer::from_seconds(RECONNECT_INTERVAL_SECS, TimerMode::Once);
                        next_state.set(MainState::ReconnectingGameRoom);
                    }
                    MainState::ReconnectingGameRoom => {
                        reconnect.timer = Timer::from_seconds(RECONNECT_INTERVAL_SECS, TimerMode::Once);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn complete_game_loading(
    state: Res<State<MainState>>,
    active_game: Res<ActiveGame>,
    mut next_state: ResMut<NextState<MainState>>,
) {
    if *state.get() != MainState::GameLoading {
        return;
    }

    if active_game.kind.is_some() {
        next_state.set(MainState::Game);
    } else {
        next_state.set(MainState::GameRoom);
    }
}

fn update_room_snapshot(room_snapshot: &mut RoomSnapshot, room_update: RoomUpdate) {
    room_snapshot.room = Some(room_update.room);

    for event in room_update.events {
        room_snapshot.messages.push(format!("{:?}", event));
    }

    if room_snapshot.messages.len() > 200 {
        let overflow = room_snapshot.messages.len() - 200;
        room_snapshot.messages.drain(..overflow);
    }
}

fn spawn_connection_runtime(
    server_url: String,
    room_path: String,
) -> (
    UnboundedSender<NetworkCommand>,
    Arc<Mutex<mpsc::Receiver<RuntimeEvent>>>,
) {
    let (command_tx, command_rx) = unbounded_channel::<NetworkCommand>();
    let (event_tx, event_rx) = mpsc::channel::<RuntimeEvent>();

    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = event_tx.send(RuntimeEvent::ConnectFailed(format!(
                    "failed to build async runtime: {error}"
                )));
                return;
            }
        };

        runtime.block_on(async move {
            run_connection_task(server_url, room_path, command_rx, event_tx).await;
        });
    });

    (command_tx, Arc::new(Mutex::new(event_rx)))
}

async fn run_connection_task(
    server_url: String,
    room_path: String,
    mut command_rx: UnboundedReceiver<NetworkCommand>,
    event_tx: mpsc::Sender<RuntimeEvent>,
) {
    let key_pair = match default_user_dir().and_then(|dir| load_first_or_create_random(&dir)) {
        Ok(key_pair) => key_pair,
        Err(error) => {
            let _ = event_tx.send(RuntimeEvent::ConnectFailed(format!(
                "failed to load identity: {error}"
            )));
            return;
        }
    };

    let _ = event_tx.send(RuntimeEvent::IdentityLoaded(key_pair.to_user()));

    let token = match authenticate(&server_url, &key_pair).await {
        Ok(token) => token,
        Err(error) => {
            let _ = event_tx.send(RuntimeEvent::ConnectFailed(format!(
                "authentication failed: {error}"
            )));
            return;
        }
    };

    let user_id = key_pair.user_id().to_string();
    let nickname = key_pair.user.nickname.clone();
    let client = match RoomClient::new(server_url.clone(), room_path, token, user_id) {
        Ok(client) => client,
        Err(error) => {
            let _ = event_tx.send(RuntimeEvent::ConnectFailed(format!(
                "create room client failed: {error}"
            )));
            return;
        }
    };

    let mut sse_stream = std::pin::pin!(client.connect_sse());

    loop {
        tokio::select! {
            maybe_command = command_rx.recv() => {
                match maybe_command {
                    Some(NetworkCommand::SendAction(action)) => {
                        if let Err(error) = client.send_action(action).await {
                            let _ = event_tx.send(RuntimeEvent::ConnectFailed(format!("send action failed: {error}")));
                        }
                    }
                    Some(NetworkCommand::Disconnect) | None => {
                        let _ = client.disconnect().await;
                        let _ = event_tx.send(RuntimeEvent::Disconnected);
                        break;
                    }
                }
            }
            maybe_event = sse_stream.next() => {
                match maybe_event {
                    Some(Ok(SseEvent::Connected)) => {
                        let _ = event_tx.send(RuntimeEvent::Connected);
                        let join_action = ActionData::RoomAction(RoomActionData::Join(JoinRoom {
                            nickname: nickname.clone(),
                        }));
                        if let Err(error) = client.send_action(join_action).await {
                            let _ = event_tx.send(RuntimeEvent::ConnectFailed(format!("send join action failed: {error}")));
                        }
                    }
                    Some(Ok(SseEvent::Update(update))) => {
                        let _ = event_tx.send(RuntimeEvent::Update(update));
                    }
                    Some(Err(error)) => {
                        let _ = event_tx.send(RuntimeEvent::ConnectFailed(format!("sse error: {error}")));
                        let _ = event_tx.send(RuntimeEvent::Disconnected);
                        break;
                    }
                    None => {
                        let _ = event_tx.send(RuntimeEvent::Disconnected);
                        break;
                    }
                }
            }
        }
    }

}
