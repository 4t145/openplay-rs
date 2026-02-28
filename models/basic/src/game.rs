use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{
    message::{App, TypedData},
    room::{RoomContext, RoomView},
    user::Action,
};

pub trait Game: Send + Sync + 'static {
    fn meta(&self) -> GameMeta;
    fn handle_action(&mut self, ctx: &RoomContext, event: SequencedGameUpdate) -> GameUpdate;

    /// Generate the current game view for all positions without mutating state.
    /// Used for pushing game state to reconnecting/joining users during an active game.
    /// Returns None if no game is in progress.
    fn current_view(&self, ctx: &RoomContext) -> Option<GameUpdate>;

    /// Get the default action for a player in the current state.
    /// This is used for timeouts, auto-play, or hints.
    fn default_action(&self, _player_id: &crate::user::UserId) -> Option<Action> {
        None
    }

    /// Apply game configuration update.
    fn apply_config(&mut self, _config: TypedData) -> Result<(), String> {
        Ok(())
    }
}

pub trait GameBot<G: Game>: Send + Sync + 'static {
    fn decide(&self, game: &G, player_id: &crate::user::UserId) -> Option<Action>;
}

pub struct WithBotAction<G, B> {
    pub inner: G,
    pub bot: B,
}

impl<G, B> Game for WithBotAction<G, B>
where
    G: Game,
    B: GameBot<G> + Send + Sync + 'static,
{
    fn meta(&self) -> GameMeta {
        self.inner.meta()
    }

    fn handle_action(&mut self, ctx: &RoomContext, event: SequencedGameUpdate) -> GameUpdate {
        self.inner.handle_action(ctx, event)
    }

    fn current_view(&self, ctx: &RoomContext) -> Option<GameUpdate> {
        self.inner.current_view(ctx)
    }

    fn default_action(&self, player_id: &crate::user::UserId) -> Option<Action> {
        if let Some(action) = self.bot.decide(&self.inner, player_id) {
            Some(action)
        } else {
            self.inner.default_action(player_id)
        }
    }

    fn apply_config(&mut self, config: TypedData) -> Result<(), String> {
        self.inner.apply_config(config)
    }
}

pub type DynGame = Box<dyn Game>;

pub struct GameMeta {
    pub app: App,
    pub description: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct ClientEvent {
    pub seq: u32,
    pub message: TypedData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct GameState {
    pub version: u32,
    pub data: TypedData,
}

#[derive(Debug, Clone)]
pub struct GameUpdate {
    pub views: HashMap<RoomView, GameViewUpdate>,
    pub snapshot: GameState,
    pub commands: Vec<GameCommand>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct GameViewUpdate {
    pub events: Vec<ClientEvent>,
    pub new_view: GameState,
}

impl GameUpdate {}

#[derive(Debug, thiserror::Error)]
pub enum ServerMessageError {
    #[error("Message rejected: {0}")]
    Rejected(#[from] MessageRejection),
}

#[derive(Debug, thiserror::Error)]
#[error("Message rejected: {reason}")]
pub struct MessageRejection {
    pub reason: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Id(Arc<str>);

impl From<String> for Id {
    fn from(s: String) -> Self {
        Id(s.into())
    }
}

impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Id(s.into())
    }
}

pub struct SequencedGameUpdate {
    pub event: GameEvent,
    pub seq: u32,
}
pub enum GameEvent {
    Action(Action),
    TimerExpired(TimeExpired),
    Interval(Interval),
    GameStart,
}

pub struct TimeExpired {
    pub timer_id: Id,
}

pub struct Interval {
    pub interval_id: Id,
}

#[derive(Debug, Clone)]
pub enum GameCommand {
    CreateTimer {
        id: Id,
        duration: Duration,
    },
    CancelTimer {
        id: Id,
        duration: Duration,
    },
    CreateInterval {
        id: Id,
    },
    CancelInterval {
        id: Id,
    },
    /// Signal that the game has ended. The room should transition back to waiting state.
    GameOver,
}
