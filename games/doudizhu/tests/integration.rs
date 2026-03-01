use bytes::Bytes;
use openplay_basic::data::Data;
use openplay_basic::game::{Game, GameEvent, SequencedGameUpdate};
use openplay_basic::message::{App, DataType, TypedData};
use openplay_basic::room::{Room, RoomContext, RoomInfo, RoomPlayerPosition, RoomPlayerState};
use openplay_basic::user::{
    game_action::GameActionData, Action as UserAction, ActionData as UserActionData, ActionSource,
    User, UserId,
};
use openplay_doudizhu::{get_app, Action, DouDizhuGame, Role, Stage};

/// 用单字节值构造确定性 UserId（字节填充到32位，首字节为 id）
fn user_id_from_byte(id: u8) -> UserId {
    let mut bytes = [0u8; 32];
    bytes[0] = id;
    UserId::from_bytes(bytes)
}

fn create_player(id: u8, name: &str) -> User {
    User {
        id: user_id_from_byte(id),
        nickname: name.to_string(),
        avatar_url: None,
        is_bot: false,
    }
}

fn create_update(player_id: &UserId, action: Action, ref_version: u32) -> SequencedGameUpdate {
    let json = serde_json::to_vec(&action).unwrap();
    let typed_data = TypedData {
        r#type: DataType {
            app: get_app(),
            r#type: "action".to_string(),
        },
        codec: "json".to_string(),
        data: Data(Bytes::from(json)),
    };

    let game_action_data = GameActionData {
        message: typed_data,
        ref_version,
    };

    SequencedGameUpdate {
        seq: 1,
        event: GameEvent::Action(UserAction {
            source: ActionSource::User(player_id.clone()),
            data: UserActionData::GameAction(game_action_data),
        }),
    }
}

// Helper to extract state
fn get_state(update: &openplay_basic::game::GameUpdate) -> DouDizhuGame {
    serde_json::from_slice(&update.snapshot.data.data.0).unwrap()
}

/// Create a RoomContext with the given players seated at positions 0, 1, 2.
fn make_room_context(players: &[User]) -> RoomContext {
    let owner = players[0].clone();
    let room_info = RoomInfo {
        title: "Test Room".to_string(),
        description: None,
        id: "test".to_string(),
        owner: owner.id.clone(),
        endpoint: "test://".to_string(),
        game_config: None,
    };
    let mut room = Room::new(room_info, owner);
    for (i, p) in players.iter().enumerate() {
        room.state.players.insert(
            RoomPlayerPosition::from(i.to_string()),
            RoomPlayerState {
                id_ready: true,
                is_connected: true,
                player: p.clone(),
            },
        );
    }
    RoomContext::new(room)
}

#[test]
fn test_game_flow() {
    let p1 = create_player(1, "Alice");
    let p2 = create_player(2, "Bob");
    let p3 = create_player(3, "Charlie");

    let mut game = DouDizhuGame::new(vec![p1.clone(), p2.clone(), p3.clone()]);
    let ctx = make_room_context(&[p1.clone(), p2.clone(), p3.clone()]);

    // Start game
    game.start(&ctx);

    // We need to inspect state to proceed
    // We can just use game directly since it's local

    assert_eq!(game.stage, Stage::Bidding);
    assert_eq!(game.players.len(), 3);
    assert_eq!(game.players[0].hand.len(), 17);
    assert_eq!(game.hole_cards.len(), 3);
    assert_eq!(game.version, 1);

    // Simulate Bidding
    let first_player_idx = game.current_turn;
    let first_player_id = game.players[first_player_idx].player.id.clone();

    // First player bids 1
    let msg = create_update(&first_player_id, Action::Bid { score: 1 }, game.version);
    let update = game.handle_action(&ctx, msg);
    let state = get_state(&update);

    assert_eq!(state.highest_bid, 1);
    assert_eq!(state.landlord_idx, Some(first_player_idx));
    assert_eq!(state.version, 2);

    // Update local game
    game = state;

    // Next player bids 2
    let next_idx = (first_player_idx + 1) % 3;
    let next_player_id = game.players[next_idx].player.id.clone();

    let msg = create_update(&next_player_id, Action::Bid { score: 2 }, game.version);
    let update = game.handle_action(&ctx, msg);
    let state = get_state(&update);

    assert_eq!(state.highest_bid, 2);
    assert_eq!(state.landlord_idx, Some(next_idx));
    assert_eq!(state.version, 3);

    game = state;

    // Third player passes (score 0)
    let third_idx = (next_idx + 1) % 3;
    let third_player_id = game.players[third_idx].player.id.clone();

    let msg = create_update(&third_player_id, Action::Bid { score: 0 }, game.version);
    let update = game.handle_action(&ctx, msg);
    let state = get_state(&update);

    game = state;

    // Player 1 passes (cannot raise to 3? or decides to pass).
    // Note: Player 1 was the first bidder. It is now their turn again.
    let msg = create_update(&first_player_id, Action::Bid { score: 0 }, game.version);
    let update = game.handle_action(&ctx, msg);
    let state = get_state(&update);

    // Now Passes=2. Should end bidding. P2 (next_idx) is landlord.
    assert_eq!(state.stage, Stage::Playing);
    assert_eq!(state.landlord_idx, Some(next_idx));
    assert_eq!(state.players[next_idx].role, Role::Landlord);
    assert_eq!(state.players[next_idx].hand.len(), 20); // 17 + 3
    assert_eq!(state.current_turn, next_idx); // Landlord starts
}
