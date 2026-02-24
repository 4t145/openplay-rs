use std::{collections::VecDeque, convert::Infallible, sync::Arc};

use openplay_basic::{
    game::UpdateGameState,
    message::TypedData,
    player::{Player, PlayerAgent, player_event::PlayerEvent},
    room::GameMessage,
};
use tokio::sync::Mutex;

pub struct ProgrammedPlayerAgent {
    pub player: Player,
    pub program: Arc<dyn PlayerProgram>,
    pub pending_messages: Arc<Mutex<VecDeque<TypedData>>>,
}

impl ProgrammedPlayerAgent {
    pub fn from_arc(player: Player, program: Arc<dyn PlayerProgram>) -> Self {
        Self {
            player,
            program,
            pending_messages: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    pub fn new<P: PlayerProgram>(player: Player, program: P) -> Self {
        Self::from_arc(player, Arc::new(program))
    }
}

impl PlayerAgent for ProgrammedPlayerAgent {
    type Error = Infallible;

    fn send_room_event(
        &self,
        event: openplay_basic::room::RoomEvent,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            if let openplay_basic::room::RoomEvent::UpdateGameState(update) = event {
                let response = self.program.decide(update);
                self.pending_messages.lock().await.push_back(response);
            }
            Ok(())
        }
    }

    fn receive_player_event(
        &self,
    ) -> impl Future<Output = Result<Option<PlayerEvent>, Self::Error>> + Send {
        let player_id = self.player.id.clone();
        async move {
            let mut lock = self.pending_messages.lock().await;
            if let Some(message) = lock.pop_front() {
                Ok(Some(PlayerEvent::GameMessage(GameMessage {
                    player_id,
                    message,
                })))
            } else {
                Ok(None)
            }
        }
    }

    fn close(&self) -> impl Future<Output = ()> + Send {
        async move {}
    }
}

pub trait PlayerProgram: Send + Sync + 'static {
    fn decide(&self, update: UpdateGameState) -> TypedData;
}
