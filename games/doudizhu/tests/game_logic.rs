use bytes::Bytes;
use openplay_basic::game::Game;
use openplay_basic::message::{DataType, TypedData};
use openplay_basic::user::{User, UserId};
use openplay_basic::room::GameMessage;
use openplay_doudizhu::{Action, DouDizhuGame, Role, Stage, APP_ID, APP_REVISION};
use openplay_poker::{Card, Rank, Suit};

fn make_player(id: &str) -> User {
    User {
        id: UserId::from(Bytes::from(id.to_string())),
        nickname: id.to_string(),
        avatar_url: None,
        is_bot: false,
    }
}

fn make_message(player_id: &str, action: Action) -> GameMessage {
    let data = serde_json::to_vec(&action).unwrap();
    GameMessage {
        player_id: UserId::from(Bytes::from(player_id.to_string())),
        message: TypedData {
            r#type: DataType {
                app: openplay_basic::message::App {
                    id: APP_ID.to_string(),
                    revision: APP_REVISION,
                },
                r#type: "action".to_string(),
            },
            codec: "json".to_string(),
            data: openplay_basic::data::Data(Bytes::from(data)),
        },
    }
}

fn card(rank: Rank) -> Card {
    Card::NaturalCard(openplay_poker::NaturalCard {
        rank,
        suit: Suit::Spades, // Suit doesn't matter for logic usually
    })
}

#[test]
fn test_invalid_move_rejected() {
    let p1 = make_player("p1");
    let p2 = make_player("p2");
    let p3 = make_player("p3");

    let mut game = DouDizhuGame::new(vec![p1.clone(), p2.clone(), p3.clone()]);
    game.start();

    // 1. Force state to Playing for simplicity (bypass bidding)
    game.stage = Stage::Playing;
    game.current_turn = 0; // p1's turn
    game.players[0].role = Role::Landlord;
    game.players[1].role = Role::Peasant;
    game.players[2].role = Role::Peasant;

    // Give specific cards to p1
    game.players[0].hand = vec![card(Rank::Three), card(Rank::Four), card(Rank::Five)];

    // 2. Attempt to play cards not in hand
    let action = Action::Play {
        cards: vec![card(Rank::Ace)], // p1 doesn't have Ace
    };
    let update = game.handle_action(make_message("p1", action));

    // State should NOT change (turn should still be 0)
    let state: DouDizhuGame = serde_json::from_slice(&update.state.data.0).unwrap();
    assert_eq!(state.current_turn, 0);
    assert_eq!(state.players[0].hand.len(), 3); // Hand size unchanged

    // 3. Play valid cards
    let action = Action::Play {
        cards: vec![card(Rank::Three)],
    };
    let update = game.handle_action(make_message("p1", action));
    let state: DouDizhuGame = serde_json::from_slice(&update.state.data.0).unwrap();
    assert_eq!(state.current_turn, 1); // Turn advanced
    assert_eq!(state.players[0].hand.len(), 2);

    // 4. p2 plays invalid (rank too low)
    // p1 played 3. p2 tries to play 3 (must be strictly higher)
    game.players[1].hand = vec![card(Rank::Three), card(Rank::Five)];
    let action = Action::Play {
        cards: vec![card(Rank::Three)],
    };
    let update = game.handle_action(make_message("p2", action));
    let state: DouDizhuGame = serde_json::from_slice(&update.state.data.0).unwrap();
    assert_eq!(state.current_turn, 1); // Still p2's turn, move rejected

    // 5. p2 plays valid (Five > Three)
    let action = Action::Play {
        cards: vec![card(Rank::Five)],
    };
    let update = game.handle_action(make_message("p2", action));
    let state: DouDizhuGame = serde_json::from_slice(&update.state.data.0).unwrap();
    assert_eq!(state.current_turn, 2); // Turn advanced
}

#[test]
fn test_pass_logic() {
    let p1 = make_player("p1");
    let p2 = make_player("p2");
    let p3 = make_player("p3");

    let mut game = DouDizhuGame::new(vec![p1, p2, p3]);
    game.start();
    game.stage = Stage::Playing;
    game.current_turn = 0;
    game.players[0].hand = vec![card(Rank::Three), card(Rank::Ace)];
    game.players[1].hand = vec![card(Rank::Four)];
    game.players[2].hand = vec![card(Rank::Five)];

    // p1 plays 3
    let update = game.handle_action(make_message(
        "p1",
        Action::Play {
            cards: vec![card(Rank::Three)],
        },
    ));
    let state: DouDizhuGame = serde_json::from_slice(&update.state.data.0).unwrap();
    println!(
        "After p1 plays: turn={}, last_play={:?}",
        state.current_turn, state.last_play
    );
    assert_eq!(state.current_turn, 1);

    // p2 Passes
    let update = game.handle_action(make_message("p2", Action::Pass));
    let state: DouDizhuGame = serde_json::from_slice(&update.state.data.0).unwrap();
    println!(
        "After p2 passes: turn={}, passes={}",
        state.current_turn, state.consecutive_passes
    );
    assert_eq!(state.current_turn, 2);
    assert_eq!(state.consecutive_passes, 1);

    // p3 Passes
    let update = game.handle_action(make_message("p3", Action::Pass));
    let state: DouDizhuGame = serde_json::from_slice(&update.state.data.0).unwrap();
    // After 2 passes (p2, p3), it should be p1's turn again (the original player)
    assert_eq!(state.current_turn, 0);
    assert_eq!(state.consecutive_passes, 0); // Reset
    assert!(state.last_play.is_none()); // Board cleared
}
