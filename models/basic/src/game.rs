use crate::{message::{App, TypedData}, room::GameMessage};

pub trait Game: Send + Sync + 'static {
    fn meta(&self) -> GameMeta;
    fn handle_message(&mut self, message: GameMessage) -> UpdateGameState;
    fn start(&mut self) -> UpdateGameState;
    fn snapshot(&self) -> TypedData;
}

pub type DynGame = Box<dyn Game>;

pub struct GameMeta {
    pub app: App,
    pub description: String,
}
pub enum GameState {
    UpdateGameState,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct AcceptedMessage {
    pub seq: u32,
    #[serde(flatten)]
    pub message: GameMessage,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct UpdateGameState {
    pub messages: Vec<AcceptedMessage>,
    pub state: TypedData,
}

impl UpdateGameState {
    pub fn snapshot(state: TypedData) -> Self {
        UpdateGameState {
            messages: Vec::new(),
            state,
        }
    }
}

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
