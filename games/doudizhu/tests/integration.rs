use bytes::Bytes;
use openplay_basic::data::Data;
use openplay_basic::game::Game;
use openplay_basic::message::{App, DataType, TypedData};
use openplay_basic::user::{User, UserId};
use openplay_basic::room::GameMessage;
use openplay_doudizhu::{get_app, Action, DouDizhuGame, Role, Stage};

fn create_player(id: u8, name: &str) -> User {
    User {
        id: UserId::from(Bytes::from(vec![id])),
        nickname: name.to_string(),
        avatar_url: None,
        is_bot: false,
    }
}

fn create_message(player_id: u8, action: Action) -> GameMessage {
    let json = serde_json::to_vec(&action).unwrap();
    GameMessage {
        player_id: UserId::from(Bytes::from(vec![player_id])),
        message: TypedData {
            r#type: DataType {
                app: get_app(),
                r#type: "action".to_string(),
            },
            codec: "json".to_string(),
            data: Data(Bytes::from(json)),
        },
    }
}

#[test]
fn test_game_flow() {
    let p1 = create_player(1, "Alice");
    let p2 = create_player(2, "Bob");
    let p3 = create_player(3, "Charlie");

    let mut game = DouDizhuGame::new(vec![p1.clone(), p2.clone(), p3.clone()]);

    // Start game
    let update = game.start();
    let state: DouDizhuGame = serde_json::from_slice(&update.state.data).unwrap();

    assert_eq!(state.stage, Stage::Bidding);
    assert_eq!(state.players.len(), 3);
    assert_eq!(state.players[0].hand.len(), 17);
    assert_eq!(state.hole_cards.len(), 3);

    // Simulate Bidding
    let first_player_idx = state.current_turn;
    let first_player_id = state.players[first_player_idx].player.id.clone();

    let first_id_u8 = if first_player_id == p1.id {
        1
    } else if first_player_id == p2.id {
        2
    } else {
        3
    };

    // First player bids 1
    let msg = create_message(first_id_u8, Action::Bid { score: 1 });
    let update = game.handle_action(msg);
    let state: DouDizhuGame = serde_json::from_slice(&update.state.data).unwrap();

    assert_eq!(state.highest_bid, 1);
    assert_eq!(state.landlord_idx, Some(first_player_idx));

    // Next player bids 2
    let next_idx = (first_player_idx + 1) % 3;
    let next_player_id = state.players[next_idx].player.id.clone();
    let next_id_u8 = if next_player_id == p1.id {
        1
    } else if next_player_id == p2.id {
        2
    } else {
        3
    };

    let msg = create_message(next_id_u8, Action::Bid { score: 2 });
    let update = game.handle_action(msg);
    let state: DouDizhuGame = serde_json::from_slice(&update.state.data).unwrap();

    assert_eq!(state.highest_bid, 2);
    assert_eq!(state.landlord_idx, Some(next_idx));

    // Third player passes (score 0)
    let third_idx = (next_idx + 1) % 3;
    let third_player_id = state.players[third_idx].player.id.clone();
    let third_id_u8 = if third_player_id == p1.id {
        1
    } else if third_player_id == p2.id {
        2
    } else {
        3
    };

    let msg = create_message(third_id_u8, Action::Bid { score: 0 });
    let update = game.handle_action(msg);
    let state: DouDizhuGame = serde_json::from_slice(&update.state.data).unwrap();

    // Player 1 passes (cannot raise to 3? or decides to pass).
    let msg = create_message(first_id_u8, Action::Bid { score: 0 });
    let update = game.handle_action(msg);
    let state: DouDizhuGame = serde_json::from_slice(&update.state.data).unwrap();

    // Now Passes=2. Should end bidding. P2 is landlord.
    assert_eq!(state.stage, Stage::Playing);
    assert_eq!(state.landlord_idx, Some(next_idx));
    assert_eq!(state.players[next_idx].role, Role::Landlord);
    assert_eq!(state.players[next_idx].hand.len(), 20); // 17 + 3
    assert_eq!(state.current_turn, next_idx); // Landlord starts
}
