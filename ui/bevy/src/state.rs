use bevy::state::state::States;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum MainState {
    #[default]
    GlobalAssetsLoading,
    Lobby,
    ConnectingGameRoom,
    GameRoom,
    GameLoading,
    Game,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum OverlayState {
    #[default]
    None,
    UserManage,
    Theme,
    Settings,
    Menu,
}