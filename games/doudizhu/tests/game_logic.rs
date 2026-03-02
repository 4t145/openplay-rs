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
use openplay_poker::{Card, Rank, Suit};

fn make_player(id: UserId, nickname: &str) -> User {
    User {
        id,
        nickname: nickname.to_string(),
        avatar_url: None,
        is_bot: false,
    }
}

fn make_action_update(player_id: &UserId, action: Action, ref_version: u32) -> SequencedGameUpdate {
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

fn card(rank: Rank) -> Card {
    Card::NaturalCard(openplay_poker::NaturalCard {
        rank,
        suit: Suit::Spades, // Suit doesn't matter for logic usually
    })
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
    let mut room = Room::new(room_info);
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

// Helper to extract state from GameUpdate
fn get_state(update: &openplay_basic::game::GameUpdate) -> DouDizhuGame {
    serde_json::from_slice(&update.snapshot.data.data.0).unwrap()
}

#[test]
fn test_invalid_move_rejected() {
    let p1 = make_player(UserId::random(), "p1");
    let p2 = make_player(UserId::random(), "p2");
    let p3 = make_player(UserId::random(), "p3");

    let mut game = DouDizhuGame::new(vec![p1.clone(), p2.clone(), p3.clone()]);
    // Manually init minimal state for testing logic
    game.stage = Stage::Playing;
    game.current_turn = 0; // p1's turn
    game.players[0].role = Role::Landlord;
    game.players[1].role = Role::Peasant;
    game.players[2].role = Role::Peasant;
    game.version = 1; // Start version

    // Give specific cards to p1
    game.players[0].hand = vec![card(Rank::Three), card(Rank::Four), card(Rank::Five)];

    let ctx = make_room_context(&[p1.clone(), p2.clone(), p3.clone()]);

    // 2. Attempt to play cards not in hand
    let action = Action::Play {
        cards: vec![card(Rank::Ace)], // p1 doesn't have Ace
    };
    let update = game.handle_action(&ctx, make_action_update(&p1.id, action, 1));

    // State should NOT change (turn should still be 0)
    let state = get_state(&update);
    assert_eq!(state.current_turn, 0);
    assert_eq!(state.players[0].hand.len(), 3); // Hand size unchanged
    assert_eq!(state.version, 1);

    // 3. Play valid cards
    let action = Action::Play {
        cards: vec![card(Rank::Three)],
    };
    let update = game.handle_action(&ctx, make_action_update(&p1.id, action, 1));
    let state = get_state(&update);
    assert_eq!(state.current_turn, 1); // Turn advanced
    assert_eq!(state.players[0].hand.len(), 2);
    assert_eq!(state.version, 2);

    // Update local game instance for next step
    game = state;

    // 4. p2 plays invalid (rank too low)
    // p1 played 3. p2 tries to play 3 (must be strictly higher)
    game.players[1].hand = vec![card(Rank::Three), card(Rank::Five)];
    let action = Action::Play {
        cards: vec![card(Rank::Three)],
    };
    // Correct version is 2 now
    let update = game.handle_action(&ctx, make_action_update(&p2.id, action, 2));
    let state = get_state(&update);
    assert_eq!(state.current_turn, 1); // Still p2's turn, move rejected
    assert_eq!(state.version, 2);

    // 5. p2 plays valid (Five > Three)
    let action = Action::Play {
        cards: vec![card(Rank::Five)],
    };
    let update = game.handle_action(&ctx, make_action_update(&p2.id, action, 2));
    let state = get_state(&update);
    assert_eq!(state.current_turn, 2); // Turn advanced
    assert_eq!(state.version, 3);
}

#[test]
fn test_pass_logic() {
    let p1 = make_player(UserId::random(), "p1");
    let p2 = make_player(UserId::random(), "p2");
    let p3 = make_player(UserId::random(), "p3");

    let mut game = DouDizhuGame::new(vec![p1.clone(), p2.clone(), p3.clone()]);
    let ctx = make_room_context(&[p1.clone(), p2.clone(), p3.clone()]);
    game.start(&ctx); // Helper to init deck etc

    // Override for specific test scenario
    game.stage = Stage::Playing;
    game.current_turn = 0;
    game.players[0].hand = vec![card(Rank::Three), card(Rank::Ace)];
    game.players[1].hand = vec![card(Rank::Four)];
    game.players[2].hand = vec![card(Rank::Five)];
    game.version = 10; // Arbitrary start

    // p1 plays 3
    let update = game.handle_action(
        &ctx,
        make_action_update(
            &p1.id,
            Action::Play {
                cards: vec![card(Rank::Three)],
            },
            10,
        ),
    );
    let state = get_state(&update);
    assert_eq!(state.current_turn, 1);

    game = state;

    // p2 Passes
    let update = game.handle_action(&ctx, make_action_update(&p2.id, Action::Pass, 11));
    let state = get_state(&update);
    assert_eq!(state.current_turn, 2);
    assert_eq!(state.consecutive_passes, 1);

    game = state;

    // p3 Passes
    let update = game.handle_action(&ctx, make_action_update(&p3.id, Action::Pass, 12));
    let state = get_state(&update);
    // After 2 passes (p2, p3), it should be p1's turn again (the original player)
    assert_eq!(state.current_turn, 0);
    assert_eq!(state.consecutive_passes, 0); // Reset
    assert!(state.last_play.is_none()); // Board cleared
}

#[test]
fn test_optimistic_locking() {
    let p1 = make_player(UserId::random(), "p1");
    let p2 = make_player(UserId::random(), "p2");
    let p3 = make_player(UserId::random(), "p3");
    let mut game = DouDizhuGame::new(vec![p1.clone(), p2.clone(), p3.clone()]);
    game.stage = Stage::Playing;
    game.players[0].hand = vec![card(Rank::Three)];
    game.version = 5;

    let ctx = make_room_context(&[p1.clone(), p2.clone(), p3]);

    // Attempt action with stale version (4)
    let action = Action::Play {
        cards: vec![card(Rank::Three)],
    };
    let update = game.handle_action(&ctx, make_action_update(&p1.id, action.clone(), 4));

    let state = get_state(&update);
    assert_eq!(state.version, 5); // Unchanged

    // Attempt action with correct version (5)
    let update = game.handle_action(&ctx, make_action_update(&p1.id, action, 5));
    let state = get_state(&update);
    assert_eq!(state.version, 6); // Changed
}
