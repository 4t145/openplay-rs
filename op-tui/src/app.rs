use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use openplay_basic::{
    data::Data,
    message::TypedData,
    room::{Room, RoomObserverView, RoomPlayerPosition, RoomUserPosition, Update},
    user::{
        game_action::GameActionData,
        room_action::{
            AddBot, KickOut, PositionChange, ReadyStateChange, RoomActionData, RoomManage,
        },
        ActionData,
    },
};
use openplay_doudizhu::{self as ddz, DouDizhuGame};
use openplay_poker::Card;

use crate::i18n;
use crate::log_buffer::LogBuffer;
use crate::user_identity::{create_identity, delete_identity, load_identities, IdentityProfile};

/// What the main loop should do after handling a key event.
pub enum KeyAction {
    /// No action needed.
    None,
    /// Send this action to the server.
    SendAction(ActionData),
    /// User wants to connect from the lobby (Enter pressed).
    Connect,
    /// User wants to cancel connecting or disconnect from game.
    Disconnect,
    /// User wants to quit the application.
    Quit,
}

/// Top-level screen state.
pub enum Screen {
    Lobby(LobbyState),
    Connecting,
    Game(GameState),
    /// Reconnecting after a disconnect during an active game.
    /// Preserves the GameState so the user can see the last known state.
    Reconnecting(ReconnectingState),
    UserManager(UserManagerState),
}

/// State for the reconnecting screen.
pub struct ReconnectingState {
    /// Preserved game state from before disconnect.
    pub game_state: GameState,
    /// Server URL to reconnect to.
    pub server_url: String,
    /// Room path for reconnection.
    pub room_path: String,
    /// Number of reconnection attempts so far.
    pub attempts: u32,
    /// Maximum number of reconnection attempts before giving up.
    pub max_attempts: u32,
    /// Error message from the last failed attempt, if any.
    pub last_error: Option<String>,
}

/// Log panel display mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogMode {
    /// Log panel hidden.
    Off,
    /// Log panel shown at the bottom (1/3 height).
    Panel,
    /// Log panel covers the entire screen.
    Fullscreen,
}

/// Lobby: user inputs server URL and user ID to connect.
pub struct LobbyState {
    pub server_url: String,
    pub user_id: String,
    /// Which field is focused: 0 = server_url, 1 = user_id
    pub focus: usize,
    pub error_message: Option<String>,
    pub selected_identity: Option<IdentityProfile>,
}

pub struct UserManagerState {
    pub profiles: Vec<IdentityProfile>,
    pub selected: usize,
    pub input: String,
    pub mode: UserManagerMode,
    pub error_message: Option<String>,
    pub previous_screen: Box<Screen>,
}

pub enum UserManagerMode {
    Browse,
    Create,
    DeleteConfirm,
}

/// In-game state.
pub struct GameState {
    /// Latest decoded doudizhu game snapshot (masked for this player).
    pub game: Option<DouDizhuGame>,
    /// Current game state version (for optimistic locking on actions).
    pub version: u32,
    /// Room info.
    pub room: Option<Room>,
    /// Our player index in the game (0-2), determined by matching user_id.
    pub my_index: Option<usize>,
    /// Currently selected card indices (in hand).
    pub selected: Vec<usize>,
    /// Cursor position in hand.
    pub cursor: usize,
    /// Whether we are in "bid mode" (pressed B, waiting for 0-3).
    pub bid_mode: bool,
    /// Whether we are in "add bot mode" (pressed A, waiting for 1-3).
    pub add_bot_mode: bool,
    /// Whether we are in "kick mode" (pressed K, waiting for 1-3).
    pub kick_mode: bool,
    /// Event/message log for display.
    pub messages: Vec<String>,
    /// Our user_id string for matching.
    pub my_user_id: String,
    /// Whether to show the right-side message panel during game.
    pub show_panel: bool,
}

impl GameState {
    pub fn new(user_id: &str) -> Self {
        Self {
            game: None,
            version: 0,
            room: None,
            my_index: None,
            selected: Vec::new(),
            cursor: 0,
            bid_mode: false,
            add_bot_mode: false,
            kick_mode: false,
            messages: Vec::new(),
            my_user_id: user_id.to_string(),
            show_panel: true,
        }
    }

    /// Determine our player index from the game state by matching user_id.
    fn detect_my_index(&mut self) {
        if let Some(ref game) = self.game {
            for (i, ps) in game.players.iter().enumerate() {
                // UserId now serializes as a JSON string
                let player_id_str: String = serde_json::to_value(&ps.player.id)
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_default();

                if player_id_str == self.my_user_id {
                    self.my_index = Some(i);
                    return;
                }
            }
        }
    }

    /// Get our hand cards (if we are a player and game is active).
    pub fn my_hand(&self) -> &[Card] {
        if let (Some(idx), Some(ref game)) = (self.my_index, &self.game) {
            if idx < game.players.len() {
                return &game.players[idx].hand;
            }
        }
        &[]
    }

    /// Is it currently our turn?
    pub fn is_my_turn(&self) -> bool {
        match (self.my_index, &self.game) {
            (Some(idx), Some(game)) => game.current_turn == idx,
            _ => false,
        }
    }

    /// Push a message to the log (keep last 50).
    pub fn push_message(&mut self, msg: String) {
        self.messages.push(msg);
        if self.messages.len() > 50 {
            self.messages.remove(0);
        }
    }

    /// Check if the current user is ready (as a seated player in the room).
    /// Returns None if user is not seated or no room info.
    pub fn my_ready_state(&self) -> Option<bool> {
        let room = self.room.as_ref()?;
        for ps in room.state.players.values() {
            // UserId now serializes as a JSON string
            let player_id_str: String = serde_json::to_value(&ps.player.id)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();
            if player_id_str == self.my_user_id {
                return Some(ps.id_ready);
            }
        }
        None
    }
}

/// The main application state.
pub struct App {
    pub screen: Screen,
    pub client: Option<openplay_client::RoomClient>,
    /// User ID for the pending/active connection, stored so we can transition
    /// from Connecting -> Game when ServerConnected arrives.
    pub pending_user_id: Option<String>,
    /// Key file to use for the pending connection (selected identity)
    pub pending_key_file: Option<String>,
    /// Nickname to use for Join (from identity file)
    pub pending_nickname: Option<String>,
    /// Key file used for the current session (for reconnect)
    pub current_key_file: Option<String>,
    /// Nickname used for the current session (for reconnect)
    pub current_nickname: Option<String>,
    /// In-memory log buffer for the TUI log panel.
    pub log_buffer: LogBuffer,
    /// Current log panel display mode.
    pub log_mode: LogMode,
    /// Scroll offset for fullscreen log view (0 = bottom / most recent).
    pub log_scroll: usize,
}

impl App {
    pub fn new(server_url: String, user_id: Option<String>, log_buffer: LogBuffer) -> Self {
        let (selected_identity, user_id_value) = if let Some(uid) = user_id {
            (None, uid)
        } else if let Some(profile) = default_identity() {
            (Some(profile.clone()), profile.user_id.clone())
        } else {
            (None, String::new())
        };
        Self {
            screen: Screen::Lobby(LobbyState {
                server_url,
                user_id: user_id_value,
                focus: 0,
                error_message: None,
                selected_identity,
            }),
            client: None,
            pending_user_id: None,
            pending_key_file: None,
            pending_nickname: None,
            current_key_file: None,
            current_nickname: None,
            log_buffer,
            log_mode: LogMode::Off,
            log_scroll: 0,
        }
    }

    /// Process a server update.
    pub fn handle_server_update(&mut self, update: Update) {
        // Handle updates in both Game and Reconnecting screens
        let gs = match &mut self.screen {
            Screen::Game(ref mut gs) => gs,
            Screen::Reconnecting(ref mut rs) => &mut rs.game_state,
            _ => return,
        };

        match update {
            Update::Room(room_update) => {
                // Check if room transitioned back to Waiting while we were in a game
                if gs.game.is_some()
                    && matches!(
                        room_update.room.state.phase.kind,
                        openplay_basic::room::RoomPhaseKind::Waiting
                    )
                {
                    // If the game is finished, keep displaying it so the user can see results.
                    // The game state will be cleared when:
                    // - A new GameView arrives (next game starts), or
                    // - The user presses Enter to dismiss the game-over screen.
                    let is_finished = gs
                        .game
                        .as_ref()
                        .is_some_and(|g| matches!(g.stage, ddz::Stage::Finished));
                    if !is_finished {
                        gs.game = None;
                        gs.selected.clear();
                        gs.cursor = 0;
                        gs.bid_mode = false;
                    }
                }
                gs.room = Some(room_update.room);
                for event in &room_update.events {
                    gs.push_message(format!("{:?}", event));
                }
            }
            Update::GameView(gv_update) => {
                gs.version = gv_update.new_view.version;

                if let Some(game) = decode_doudizhu_state(&gv_update.new_view.data) {
                    gs.game = Some(game);
                    gs.detect_my_index();

                    // Reset selection when game state changes
                    gs.selected.clear();
                    let hand_len = gs.my_hand().len();
                    if gs.cursor >= hand_len && hand_len > 0 {
                        gs.cursor = hand_len - 1;
                    }
                }

                for event in &gv_update.events {
                    gs.push_message(format!("Game event seq={}", event.seq));
                }
            }
        }
    }

    /// Handle a key event and return what the main loop should do.
    pub fn handle_key(&mut self, key: KeyEvent) -> KeyAction {
        // Global: Ctrl+C always quits
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return KeyAction::Quit;
        }

        // Global: Ctrl+U opens user manager
        if key.code == KeyCode::Char('u') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if !matches!(self.screen, Screen::UserManager(_)) {
                self.open_user_manager();
            }
            return KeyAction::None;
        }

        // Global: F12 toggles log panel mode (Off -> Panel -> Fullscreen -> Off)
        if key.code == KeyCode::F(12) {
            self.log_mode = match self.log_mode {
                LogMode::Off => LogMode::Panel,
                LogMode::Panel => LogMode::Fullscreen,
                LogMode::Fullscreen => LogMode::Off,
            };
            self.log_scroll = 0;
            return KeyAction::None;
        }

        // Fullscreen log mode intercepts navigation keys
        if self.log_mode == LogMode::Fullscreen {
            match key.code {
                KeyCode::Esc => {
                    self.log_mode = LogMode::Off;
                    self.log_scroll = 0;
                    return KeyAction::None;
                }
                KeyCode::Up => {
                    self.log_scroll = self.log_scroll.saturating_add(1);
                    return KeyAction::None;
                }
                KeyCode::Down => {
                    self.log_scroll = self.log_scroll.saturating_sub(1);
                    return KeyAction::None;
                }
                KeyCode::Home => {
                    // Scroll to oldest
                    self.log_scroll = self.log_buffer.len().saturating_sub(1);
                    return KeyAction::None;
                }
                KeyCode::End => {
                    // Scroll to newest
                    self.log_scroll = 0;
                    return KeyAction::None;
                }
                _ => {
                    // Ignore other keys in fullscreen log mode
                    return KeyAction::None;
                }
            }
        }

        match &mut self.screen {
            Screen::Lobby(lobby) => {
                match key.code {
                    KeyCode::Enter => {
                        if !lobby.server_url.is_empty() && !lobby.user_id.is_empty() {
                            return KeyAction::Connect;
                        }
                    }
                    _ => handle_lobby_key(lobby, key),
                }
                KeyAction::None
            }
            Screen::Connecting => {
                // Esc or Q cancels connecting
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => KeyAction::Disconnect,
                    _ => KeyAction::None,
                }
            }
            Screen::Reconnecting(_) => {
                // Esc or Q gives up reconnecting and returns to lobby
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => KeyAction::Disconnect,
                    _ => KeyAction::None,
                }
            }
            Screen::Game(gs) => handle_game_key(gs, key),
            Screen::UserManager(_) => {
                self.handle_user_manager_key_event(key);
                KeyAction::None
            }
        }
    }

    /// Transition to connecting state, returning (server_url, user_id) for the caller to initiate.
    pub fn start_connect(&mut self) -> Option<(String, String)> {
        if let Screen::Lobby(ref lobby) = self.screen {
            if lobby.server_url.is_empty() || lobby.user_id.is_empty() {
                return None;
            }
            let result = (lobby.server_url.clone(), lobby.user_id.clone());
            self.pending_user_id = Some(lobby.user_id.clone());
            self.pending_nickname = lobby
                .selected_identity
                .as_ref()
                .map(|profile| profile.nickname.clone());
            self.pending_key_file = lobby
                .selected_identity
                .as_ref()
                .map(|profile| profile.path.to_string_lossy().to_string());
            self.screen = Screen::Connecting;
            Some(result)
        } else {
            None
        }
    }

    /// SSE connection confirmed open. Transition from Connecting -> Game.
    pub fn connected(&mut self) {
        if let Some(user_id) = self.pending_user_id.take() {
            self.screen = Screen::Game(GameState::new(&user_id));
        }
        self.current_key_file = self.pending_key_file.take();
        self.current_nickname = self.pending_nickname.take();
    }

    /// Handle connection failure, go back to lobby with error message.
    pub fn connection_failed(&mut self, error: String) {
        self.pending_user_id = None;
        self.pending_key_file = None;
        self.pending_nickname = None;
        self.current_key_file = None;
        self.current_nickname = None;
        self.screen = Screen::Lobby(make_lobby_state(String::new(), Some(error)));
    }

    /// Cancel an in-progress connection or disconnect from game. Returns to lobby.
    pub fn go_to_lobby(&mut self, error: Option<String>) {
        self.pending_user_id = None;
        self.pending_key_file = None;
        self.pending_nickname = None;
        self.current_key_file = None;
        self.current_nickname = None;
        self.client = None;
        self.screen = Screen::Lobby(make_lobby_state(String::new(), error));
    }

    /// Enter reconnecting state, preserving game state for display.
    /// Returns (server_url, user_id, room_path) for the caller to initiate reconnection.
    pub fn go_to_reconnecting(&mut self, server_url: String, room_path: String) {
        self.client = None;
        self.pending_key_file = self.current_key_file.clone();
        self.pending_nickname = self.current_nickname.clone();
        // Take the GameState out of Screen::Game
        let old_screen = std::mem::replace(
            &mut self.screen,
            Screen::Lobby(make_lobby_state(String::new(), None)),
        );
        if let Screen::Game(gs) = old_screen {
            self.screen = Screen::Reconnecting(ReconnectingState {
                game_state: gs,
                server_url,
                room_path,
                attempts: 0,
                max_attempts: 10,
                last_error: None,
            });
        }
        // If not in Game screen, fall through to lobby (shouldn't happen)
    }

    /// Reconnection succeeded. Transition from Reconnecting -> Game.
    pub fn reconnected(&mut self) {
        let old_screen = std::mem::replace(
            &mut self.screen,
            Screen::Lobby(make_lobby_state(String::new(), None)),
        );
        if let Screen::Reconnecting(rs) = old_screen {
            self.screen = Screen::Game(rs.game_state);
        }
        self.pending_key_file = None;
        self.pending_nickname = None;
    }

    fn open_user_manager(&mut self) {
        let profiles = load_identities().unwrap_or_default();
        let selected_identity = match &self.screen {
            Screen::Lobby(lobby) => lobby
                .selected_identity
                .as_ref()
                .map(|profile| profile.user_id.clone()),
            _ => None,
        };
        let selected = selected_identity
            .and_then(|uid| profiles.iter().position(|p| p.user_id == uid))
            .unwrap_or(0);
        let previous_screen = Box::new(std::mem::replace(
            &mut self.screen,
            Screen::Lobby(make_lobby_state(String::new(), None)),
        ));
        self.screen = Screen::UserManager(UserManagerState {
            profiles,
            selected,
            input: String::new(),
            mode: UserManagerMode::Browse,
            error_message: None,
            previous_screen,
        });
    }

    fn handle_user_manager_key_event(&mut self, key: KeyEvent) {
        let screen = std::mem::replace(
            &mut self.screen,
            Screen::Lobby(make_lobby_state(String::new(), None)),
        );
        if let Screen::UserManager(mut um) = screen {
            let next_screen = handle_user_manager_key(&mut um, key);
            self.screen = next_screen.unwrap_or(Screen::UserManager(um));
        } else {
            self.screen = screen;
        }
    }
}

/// Handle key input on the lobby screen.
fn handle_lobby_key(lobby: &mut LobbyState, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => {
            lobby.focus = (lobby.focus + 1) % 2;
        }
        KeyCode::BackTab => {
            lobby.focus = if lobby.focus == 0 { 1 } else { 0 };
        }
        KeyCode::Char(c) => {
            if lobby.focus == 0 {
                lobby.server_url.push(c);
            } else {
                if lobby.selected_identity.take().is_some() {
                    lobby.user_id.clear();
                }
                lobby.user_id.push(c);
            }
        }
        KeyCode::Backspace => {
            if lobby.focus == 0 {
                lobby.server_url.pop();
            } else {
                if lobby.selected_identity.take().is_some() {
                    lobby.user_id.clear();
                } else {
                    lobby.user_id.pop();
                }
            }
        }
        _ => {}
    }
}

fn handle_user_manager_key(um: &mut UserManagerState, key: KeyEvent) -> Option<Screen> {
    let mut next_screen: Option<Screen> = None;
    match um.mode {
        UserManagerMode::Browse => match key.code {
            KeyCode::Esc => {
                let previous = std::mem::replace(
                    &mut um.previous_screen,
                    Box::new(Screen::Lobby(make_lobby_state(String::new(), None))),
                );
                next_screen = Some(*previous);
            }
            KeyCode::Up => {
                if um.selected > 0 {
                    um.selected -= 1;
                }
            }
            KeyCode::Down => {
                if um.selected + 1 < um.profiles.len() {
                    um.selected += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(profile) = um.profiles.get(um.selected).cloned() {
                    if let Screen::Lobby(ref mut lobby) = um.previous_screen.as_mut() {
                        lobby.selected_identity = Some(profile.clone());
                        lobby.user_id = profile.user_id.clone();
                    }
                    let previous = std::mem::replace(
                        &mut um.previous_screen,
                        Box::new(Screen::Lobby(make_lobby_state(String::new(), None))),
                    );
                    next_screen = Some(*previous);
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                um.mode = UserManagerMode::Create;
                um.input.clear();
                um.error_message = None;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if !um.profiles.is_empty() {
                    um.mode = UserManagerMode::DeleteConfirm;
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => match load_identities() {
                Ok(profiles) => {
                    um.profiles = profiles;
                    um.selected = um.selected.min(um.profiles.len().saturating_sub(1));
                    um.error_message = None;
                }
                Err(err) => um.error_message = Some(err),
            },
            _ => {}
        },
        UserManagerMode::Create => match key.code {
            KeyCode::Esc => {
                um.mode = UserManagerMode::Browse;
                um.input.clear();
            }
            KeyCode::Enter => {
                let nickname = um.input.trim();
                if nickname.is_empty() {
                    um.error_message = Some(i18n::t("user-manager-nickname-required"));
                } else {
                    match create_identity(nickname) {
                        Ok(profile) => {
                            let new_user_id = profile.user_id.clone();
                            um.profiles.push(profile);
                            um.profiles.sort_by(|a, b| {
                                a.nickname.cmp(&b.nickname).then(a.user_id.cmp(&b.user_id))
                            });
                            um.selected = um
                                .profiles
                                .iter()
                                .position(|p| p.user_id == new_user_id)
                                .unwrap_or(0);
                            um.mode = UserManagerMode::Browse;
                            um.input.clear();
                            um.error_message = None;
                        }
                        Err(err) => um.error_message = Some(err),
                    }
                }
            }
            KeyCode::Backspace => {
                um.input.pop();
            }
            KeyCode::Char(c) => {
                um.input.push(c);
            }
            _ => {}
        },
        UserManagerMode::DeleteConfirm => match key.code {
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                um.mode = UserManagerMode::Browse;
            }
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(profile) = um.profiles.get(um.selected).cloned() {
                    let deleted_user_id = profile.user_id.clone();
                    match delete_identity(&profile.path) {
                        Ok(_) => {
                            um.profiles.remove(um.selected);
                            if let Screen::Lobby(ref mut lobby) = um.previous_screen.as_mut() {
                                if lobby
                                    .selected_identity
                                    .as_ref()
                                    .is_some_and(|p| p.user_id == deleted_user_id)
                                {
                                    lobby.selected_identity = None;
                                    lobby.user_id.clear();
                                }
                            }
                            if um.selected >= um.profiles.len() && !um.profiles.is_empty() {
                                um.selected = um.profiles.len() - 1;
                            }
                            um.mode = UserManagerMode::Browse;
                            um.error_message = None;
                        }
                        Err(err) => um.error_message = Some(err),
                    }
                } else {
                    um.mode = UserManagerMode::Browse;
                }
            }
            _ => {}
        },
    }

    next_screen
}

fn default_identity() -> Option<IdentityProfile> {
    load_identities().ok().and_then(|mut profiles| {
        profiles.sort_by(|a, b| a.nickname.cmp(&b.nickname).then(a.user_id.cmp(&b.user_id)));
        profiles.into_iter().next()
    })
}

fn make_lobby_state(server_url: String, error_message: Option<String>) -> LobbyState {
    let selected_identity = default_identity();
    let user_id = selected_identity
        .as_ref()
        .map(|profile| profile.user_id.clone())
        .unwrap_or_default();
    LobbyState {
        server_url,
        user_id,
        focus: 0,
        error_message,
        selected_identity,
    }
}

/// Handle key input on the game screen. Returns what the main loop should do.
fn handle_game_key(gs: &mut GameState, key: KeyEvent) -> KeyAction {
    // Q or Esc: disconnect and return to lobby
    if key.code == KeyCode::Char('q') || key.code == KeyCode::Char('Q') || key.code == KeyCode::Esc
    {
        // Don't intercept Q/Esc while in a sub-mode — cancel the mode instead
        if gs.bid_mode {
            gs.bid_mode = false;
            return KeyAction::None;
        }
        if gs.add_bot_mode {
            gs.add_bot_mode = false;
            return KeyAction::None;
        }
        if gs.kick_mode {
            gs.kick_mode = false;
            return KeyAction::None;
        }
        return KeyAction::Disconnect;
    }

    // Bid mode: B was pressed, waiting for 0-3
    if gs.bid_mode {
        gs.bid_mode = false;
        match key.code {
            KeyCode::Char(c @ '0'..='3') => {
                let score = c.to_digit(10).unwrap() as u8;
                return KeyAction::SendAction(make_game_action(
                    ddz::Action::Bid { score },
                    gs.version,
                ));
            }
            _ => return KeyAction::None, // Cancel bid mode
        }
    }

    // Add-bot mode: A was pressed, waiting for 1-3
    if gs.add_bot_mode {
        gs.add_bot_mode = false;
        match key.code {
            KeyCode::Char(c @ '1'..='3') => {
                let seat = (c.to_digit(10).unwrap() - 1).to_string();
                return KeyAction::SendAction(ActionData::RoomAction(RoomActionData::RoomManage(
                    RoomManage::AddBot(AddBot {
                        position: RoomPlayerPosition::from(seat.as_str()),
                        name: None,
                    }),
                )));
            }
            _ => return KeyAction::None,
        }
    }

    // Kick mode: K was pressed, waiting for 1-3
    if gs.kick_mode {
        gs.kick_mode = false;
        match key.code {
            KeyCode::Char(c @ '1'..='3') => {
                let seat = (c.to_digit(10).unwrap() - 1).to_string();
                let pos = RoomPlayerPosition::from(seat.as_str());
                // Find the player at this seat
                if let Some(ref room) = gs.room {
                    if let Some(player_state) = room.state.players.get(&pos) {
                        return KeyAction::SendAction(ActionData::RoomAction(
                            RoomActionData::RoomManage(RoomManage::KickOut(KickOut {
                                player: player_state.player.id.clone(),
                                reason: None,
                                ban: None,
                            })),
                        ));
                    }
                }
                return KeyAction::None;
            }
            _ => return KeyAction::None,
        }
    }

    // --- Game over screen: stage is Finished, room is back to Waiting ---
    // Allow R (ready), S (start), Enter (dismiss to waiting room), Q (disconnect).
    if let Some(ref game) = gs.game {
        if matches!(game.stage, ddz::Stage::Finished) {
            match key.code {
                // R: toggle ready for next game
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    let new_ready = !gs.my_ready_state().unwrap_or(false);
                    return KeyAction::SendAction(ActionData::RoomAction(
                        RoomActionData::ChangeReadyState(ReadyStateChange {
                            is_ready: new_ready,
                        }),
                    ));
                }
                // S: start next game (owner only)
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    return KeyAction::SendAction(ActionData::RoomAction(
                        RoomActionData::RoomManage(RoomManage::StartGame),
                    ));
                }
                // Enter: dismiss game-over screen, go back to waiting room view
                KeyCode::Enter => {
                    gs.game = None;
                    gs.selected.clear();
                    gs.cursor = 0;
                    gs.bid_mode = false;
                    return KeyAction::None;
                }
                // Tab: toggle panel
                KeyCode::Tab => {
                    gs.show_panel = !gs.show_panel;
                }
                _ => {}
            }
            return KeyAction::None;
        }
    }

    // --- Waiting phase (no game yet): room management keys ---
    if gs.game.is_none() {
        match key.code {
            // 1-3: sit at seat (observer -> player)
            KeyCode::Char(c @ '1'..='3') => {
                let seat = (c.to_digit(10).unwrap() - 1).to_string();
                return KeyAction::SendAction(ActionData::RoomAction(
                    RoomActionData::PositionChange(PositionChange {
                        from: RoomUserPosition::Observer(RoomObserverView::default()),
                        to: RoomUserPosition::Player(RoomPlayerPosition::from(seat.as_str())),
                    }),
                ));
            }

            // A: enter add-bot mode
            KeyCode::Char('a') | KeyCode::Char('A') => {
                gs.add_bot_mode = true;
            }

            // K: enter kick mode
            KeyCode::Char('k') | KeyCode::Char('K') => {
                gs.kick_mode = true;
            }

            // Ready toggle
            KeyCode::Char('r') | KeyCode::Char('R') => {
                let new_ready = !gs.my_ready_state().unwrap_or(false);
                return KeyAction::SendAction(ActionData::RoomAction(
                    RoomActionData::ChangeReadyState(ReadyStateChange {
                        is_ready: new_ready,
                    }),
                ));
            }

            // S: start game (owner only, server validates)
            KeyCode::Char('s') | KeyCode::Char('S') => {
                return KeyAction::SendAction(ActionData::RoomAction(RoomActionData::RoomManage(
                    RoomManage::StartGame,
                )));
            }

            _ => {}
        }
        return KeyAction::None;
    }

    // --- Active game phase ---
    match key.code {
        // Tab: toggle message panel
        KeyCode::Tab => {
            gs.show_panel = !gs.show_panel;
        }

        // Ready toggle (also available during game for re-match scenarios)
        KeyCode::Char('r') | KeyCode::Char('R') => {
            let new_ready = !gs.my_ready_state().unwrap_or(false);
            return KeyAction::SendAction(ActionData::RoomAction(
                RoomActionData::ChangeReadyState(ReadyStateChange {
                    is_ready: new_ready,
                }),
            ));
        }

        // Enter bid mode
        KeyCode::Char('b') | KeyCode::Char('B') => {
            if let Some(ref game) = gs.game {
                if matches!(game.stage, ddz::Stage::Bidding) && gs.is_my_turn() {
                    gs.bid_mode = true;
                }
            }
        }

        // Navigate hand
        KeyCode::Left => {
            if gs.cursor > 0 {
                gs.cursor -= 1;
            }
        }
        KeyCode::Right => {
            let hand_len = gs.my_hand().len();
            if hand_len > 0 && gs.cursor < hand_len - 1 {
                gs.cursor += 1;
            }
        }

        // Toggle card selection
        KeyCode::Char(' ') => {
            let hand_len = gs.my_hand().len();
            if gs.cursor < hand_len {
                if let Some(pos) = gs.selected.iter().position(|&i| i == gs.cursor) {
                    gs.selected.remove(pos);
                } else {
                    gs.selected.push(gs.cursor);
                }
            }
        }

        // Play selected cards
        KeyCode::Enter => {
            if gs.is_my_turn() && !gs.selected.is_empty() {
                if let Some(ref game) = gs.game {
                    if matches!(game.stage, ddz::Stage::Playing) {
                        let hand = gs.my_hand();
                        let cards: Vec<Card> = gs
                            .selected
                            .iter()
                            .filter_map(|&i| hand.get(i).cloned())
                            .collect();
                        gs.selected.clear();
                        return KeyAction::SendAction(make_game_action(
                            ddz::Action::Play { cards },
                            gs.version,
                        ));
                    }
                }
            }
        }

        // Pass
        KeyCode::Char('p') | KeyCode::Char('P') => {
            if gs.is_my_turn() {
                if let Some(ref game) = gs.game {
                    if matches!(game.stage, ddz::Stage::Playing) {
                        return KeyAction::SendAction(make_game_action(
                            ddz::Action::Pass,
                            gs.version,
                        ));
                    }
                }
            }
        }

        _ => {}
    }

    KeyAction::None
}

/// Decode DouDizhuGame from a TypedData envelope.
fn decode_doudizhu_state(typed_data: &TypedData) -> Option<DouDizhuGame> {
    let bytes: &[u8] = &typed_data.data.0;
    match serde_json::from_slice::<DouDizhuGame>(bytes) {
        Ok(game) => Some(game),
        Err(e) => {
            tracing::warn!("Failed to decode doudizhu state: {}", e);
            None
        }
    }
}

/// Construct a GameAction ActionData from a doudizhu Action.
fn make_game_action(action: ddz::Action, ref_version: u32) -> ActionData {
    let action_json = serde_json::to_vec(&action).expect("Failed to serialize action");
    let message = TypedData {
        r#type: openplay_basic::message::DataType {
            app: ddz::get_app(),
            r#type: "action".to_string(),
        },
        codec: "json".to_string(),
        data: Data(bytes::Bytes::from(action_json)),
    };
    ActionData::GameAction(GameActionData {
        message,
        ref_version,
    })
}
