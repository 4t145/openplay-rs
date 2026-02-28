use std::{collections::VecDeque, convert::Infallible, sync::Arc};

use openplay_basic::{
    game::GameViewUpdate,
    message::TypedData,
    room::Update,
    user::{ActionData, User, UserAgent, game_action::GameActionData},
};
use tokio::sync::{Mutex, Notify};

pub struct ProgrammedUserAgent {
    pub user: User,
    pub program: Arc<dyn UserProgram>,
    pub pending_actions: Arc<Mutex<VecDeque<ActionData>>>,
    pub notify: Arc<Notify>,
    pub last_processed_version: Arc<Mutex<Option<u32>>>,
}

impl ProgrammedUserAgent {
    pub fn from_arc(user: User, program: Arc<dyn UserProgram>) -> Self {
        Self {
            user,
            program,
            pending_actions: Arc::new(Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
            last_processed_version: Arc::new(Mutex::new(None)),
        }
    }
    pub fn new<P: UserProgram>(user: User, program: P) -> Self {
        Self::from_arc(user, Arc::new(program))
    }
}

impl UserAgent for ProgrammedUserAgent {
    type Error = Infallible;

    fn send_update(&self, update: Update) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let program = self.program.clone();
        let pending_actions = self.pending_actions.clone();
        let notify = self.notify.clone();
        let last_processed_version = self.last_processed_version.clone();
        async move {
            if let Update::GameView(game_view_update) = update {
                // Bots only care about game updates usually, but could be extended
                let current_version = game_view_update.new_view.version;
                
                // Check for duplicate updates (but allow version reset on new game)
                let mut last_version_guard = last_processed_version.lock().await;
                if let Some(last) = *last_version_guard {
                    if current_version == last {
                        // Exact duplicate — ignore
                        return Ok(());
                    }
                    // If current_version < last, a new game started (version reset).
                    // Accept it; don't treat it as outdated.
                }
                *last_version_guard = Some(current_version);
                drop(last_version_guard); // Release lock early

                if let Some(decision_message) = program.decide(&game_view_update) {
                    let action_data = ActionData::GameAction(GameActionData {
                        message: decision_message,
                        ref_version: current_version,
                    });
                    pending_actions.lock().await.push_back(action_data);
                    notify.notify_one();
                }
            }
            Ok(())
        }
    }

    fn receive_action(
        &self,
    ) -> impl Future<Output = Result<Option<ActionData>, Self::Error>> + Send {
        let pending_actions = self.pending_actions.clone();
        let notify = self.notify.clone();
        async move {
            loop {
                let mut lock = pending_actions.lock().await;
                if let Some(action) = lock.pop_front() {
                    return Ok(Some(action));
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

pub trait UserProgram: Send + Sync + 'static {
    fn decide(&self, update: &GameViewUpdate) -> Option<TypedData>;
}
