use std::{collections::VecDeque, convert::Infallible, sync::Arc};

use openplay_basic::{
    game::UpdateGameState,
    message::TypedData,
    user::{User, PlayerAgent, player_event::PlayerEvent},
    room::GameMessage,
};
use tokio::sync::{Mutex, Notify};

pub struct ProgrammedPlayerAgent {
    pub player: User,
    pub program: Arc<dyn PlayerProgram>,
    pub pending_messages: Arc<Mutex<VecDeque<TypedData>>>,
    pub notify: Arc<Notify>,
}

impl ProgrammedPlayerAgent {
    pub fn from_arc(player: User, program: Arc<dyn PlayerProgram>) -> Self {
        Self {
            player,
            program,
            pending_messages: Arc::new(Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
        }
    }
    pub fn new<P: PlayerProgram>(player: User, program: P) -> Self {
        Self::from_arc(player, Arc::new(program))
    }
}

impl PlayerAgent for ProgrammedPlayerAgent {
    type Error = Infallible;

    fn send_room_event(
        &self,
        event: openplay_basic::room::RoomEvent,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let program = self.program.clone();
        let pending_messages = self.pending_messages.clone();
        let notify = self.notify.clone();
        async move {
            if let openplay_basic::room::RoomEvent::UpdateGameState(update) = event {
                if let Some(response) = program.decide(update) {
                    pending_messages.lock().await.push_back(response);
                    notify.notify_one();
                }
            }
            Ok(())
        }
    }

    fn receive_player_event(
        &self,
    ) -> impl Future<Output = Result<Option<PlayerEvent>, Self::Error>> + Send {
        let player_id = self.player.id.clone();
        let pending_messages = self.pending_messages.clone();
        let notify = self.notify.clone();
        async move {
            loop {
                let mut lock = pending_messages.lock().await;
                if let Some(message) = lock.pop_front() {
                    return Ok(Some(PlayerEvent::GameMessage(GameMessage {
                        player_id: player_id.clone(),
                        message,
                    })));
                }
                drop(lock);
                notify.notified().await;
            }
        }
    }

    fn close(&self) -> impl Future<Output = ()> + Send {
        async move {}
    }
}

pub trait PlayerProgram: Send + Sync + 'static {
    fn decide(&self, update: UpdateGameState) -> Option<TypedData>;
}
