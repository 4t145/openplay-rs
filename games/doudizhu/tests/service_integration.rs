use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use bytes::Bytes;
use openplay_basic::{
    data::Data,
    game::GameViewUpdate,
    message::{DataType, TypedData},
    user::{
        ActionData, DynUserAgent, User, UserAgent, UserId,
        room_action::{ReadyStateChange, RoomActionData},
    },
    room::{Room, RoomInfo, RoomEvent, Update},
};
use openplay_doudizhu::{DouDizhuGame, Stage};
use openplay_host::service::RoomService;
use openplay_pa_programmed::{PlayerProgram, ProgrammedPlayerAgent};
use tokio::sync::Notify;
use tracing::info;

struct DouDizhuBot {
    player_id: UserId,
    finished_notify: Arc<Notify>,
}

impl DouDizhuBot {
    fn new(player_id: UserId, finished_notify: Arc<Notify>) -> Self {
        Self {
            player_id,
            finished_notify,
        }
    }
}

impl PlayerProgram for DouDizhuBot {
    fn decide(&self, update: &GameViewUpdate) -> Option<TypedData> {
        // 1. Deserialize game state
        let game: DouDizhuGame = serde_json::from_slice(&update.new_view.data.data.0).unwrap();

        // Check if finished
        if let Stage::Finished = game.stage {
            info!(
                "Bot {} detected game finished. Winner: {:?}",
                self.player_id, game.winner
            );
            self.finished_notify.notify_waiters();
            return None;
        }

        // Use shared bot logic
        if let Some(action) = openplay_doudizhu::bot::SimpleBotLogic::decide(&self.player_id, &game) {
            info!("Bot {} decided action: {:?}", self.player_id, action);
            
            // Just return the Action. ProgrammedPlayerAgent will wrap it in GameActionData.
            let json = serde_json::to_string(&action).unwrap();
            Some(TypedData {
                r#type: DataType {
                    app: openplay_doudizhu::get_app(),
                    r#type: "action".to_string(),
                },
                codec: "json".to_string(),
                data: Data(Bytes::from(json)),
            })
        } else {
            None
        }
    }
}

struct StartGameAgent {
    inner: ProgrammedPlayerAgent,
    started: AtomicBool,
}

impl StartGameAgent {
    fn new(inner: ProgrammedPlayerAgent) -> Self {
        Self {
            inner,
            started: AtomicBool::new(false),
        }
    }
}

impl UserAgent for StartGameAgent {
    type Error = <ProgrammedPlayerAgent as UserAgent>::Error;

    fn send_update(
        &self,
        event: Update,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        async move {
            self.inner.send_update(event).await
        }
    }
    
    fn receive_player_action(
        &self,
    ) -> impl std::future::Future<Output = Result<Option<ActionData>, Self::Error>> + Send {
        async move {
            if !self.started.load(Ordering::Relaxed) {
                info!("StartGameAgent sending Ready event");
                self.started.store(true, Ordering::Relaxed);
                // Send Ready State Change
                return Ok(Some(ActionData::RoomAction(RoomActionData::ChangeReadyState(
                    ReadyStateChange { is_ready: true },
                ))));
            }
            self.inner.receive_player_action().await
        }
    }

    fn close(&self) -> impl std::future::Future<Output = ()> + Send {
        self.inner.close()
    }
}

#[tokio::test]
async fn test_service_integration() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_test_writer()
        .init();

    let finished_notify = Arc::new(Notify::new());

    let p1_id = UserId::from(Bytes::from("p1"));
    let p2_id = UserId::from(Bytes::from("p2"));
    let p3_id = UserId::from(Bytes::from("p3"));

    let p1 = User::new_robot("Player 1".to_string(), p1_id.clone());
    let p2 = User::new_robot("Player 2".to_string(), p2_id.clone());
    let p3 = User::new_robot("Player 3".to_string(), p3_id.clone());

    let players = vec![p1.clone(), p2.clone(), p3.clone()];
    
    // Create Game
    let game = DouDizhuGame::new(players.clone());

    // Create Room
    let room_info = RoomInfo {
        title: "Test Room".to_string(),
        description: None,
        id: "room1".to_string(),
        owner: p1_id.clone(),
        endpoint: "ws://localhost".to_string(),
    };
    let mut room = Room::new(room_info, p1.clone());
    
    // Add other players to the room state manually for test (usually join via connection)
    // We mock room state here
    use openplay_basic::room::{RoomPlayerState, RoomPlayerPosition};
    
    // Helper to add player
    let mut add_player = |p: User, i: usize| {
        room.state.players.insert(
            RoomPlayerPosition::from(i.to_string()), 
            RoomPlayerState {
                id_ready: false, // Start unready
                is_connected: true,
                player: p,
            }
        );
    };
    
    // p1 is already added as owner/observer? No, Room::new adds owner as observer.
    // We need to add them as players explicitly if we want them to play.
    // Room::new adds owner as observer only.
    
    // Add p1, p2, p3 as players
    add_player(p1.clone(), 0);
    add_player(p2.clone(), 1);
    add_player(p3.clone(), 2);
    
    // Also mark p2 and p3 as READY. p1 will send ready via StartGameAgent.
    room.state.players.get_mut(&RoomPlayerPosition::from("1")).unwrap().id_ready = true;
    room.state.players.get_mut(&RoomPlayerPosition::from("2")).unwrap().id_ready = true;


    // 2. Create Bots
    let bot1 = DouDizhuBot::new(p1.id.clone(), finished_notify.clone());
    let bot2 = DouDizhuBot::new(p2.id.clone(), finished_notify.clone());
    let bot3 = DouDizhuBot::new(p3.id.clone(), finished_notify.clone());

    // 3. Create Agents
    let agent1 = StartGameAgent::new(ProgrammedPlayerAgent::new(p1.clone(), bot1));
    let agent2 = ProgrammedPlayerAgent::new(p2.clone(), bot2);
    let agent3 = ProgrammedPlayerAgent::new(p3.clone(), bot3);

    let mut player_agents: HashMap<UserId, DynUserAgent> = HashMap::new();
    
    use openplay_basic::user::new_dyn_user_agent; // Fixed from new_dyn_player_agent
    player_agents.insert(p1.id.clone(), new_dyn_user_agent(agent1));
    player_agents.insert(p2.id.clone(), new_dyn_user_agent(agent2));
    player_agents.insert(p3.id.clone(), new_dyn_user_agent(agent3));

    // 4. Create Service
    let service = RoomService {
        game: Box::new(game),
        room,
        player_agents,
    };

    // 5. Run
    let mut service_handle = service.run();

    // Wait for finish or timeout
    tokio::select! {
        res = &mut service_handle.join_handle => {
             match res {
                Ok(Ok(())) => println!("Service finished normally (unexpected)"),
                Ok(Err(e)) => panic!("Service error: {:?}", e),
                Err(e) => panic!("Service join error: {:?}", e),
             }
        }
        _ = finished_notify.notified() => {
            println!("Game finished successfully!");
        }
        _ = tokio::time::sleep(Duration::from_secs(30)) => {
            service_handle.cancel_token.cancel();
            panic!("Test timed out! Game did not finish in 30 seconds.");
        }
    }

    // Cleanup
    service_handle.cancel_token.cancel();
}
