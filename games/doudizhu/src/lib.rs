use crate::pattern::{analyze_pattern, DouDizhuRank, Pattern};
use bytes::Bytes;
use openplay_basic::data::Data;
use openplay_basic::game::{AcceptedMessage, Game, GameMeta, UpdateGameState};
use openplay_basic::message::{App, DataType, TypedData};
use openplay_basic::user::User;
use openplay_basic::room::GameMessage;
use openplay_poker::{Card, Deck};
use rand::Rng;
use serde::{Deserialize, Serialize};

pub mod bot;
pub mod pattern;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    Setup,
    Bidding,
    Playing,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub player: User,
    pub hand: Vec<Card>,
    pub role: Role,
    pub has_passed: bool, // In bidding or playing
    pub bid_score: u8,    // 0 for pass
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Undecided,
    Landlord,
    Peasant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastPlay {
    pub player_idx: usize,
    pub cards: Vec<Card>,
    pub pattern: Pattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DouDizhuGame {
    pub players: Vec<PlayerState>,
    pub deck: Deck,
    pub hole_cards: Vec<Card>, // Bottom 3 cards
    pub stage: Stage,
    pub current_turn: usize, // Index of player whose turn it is
    pub landlord_idx: Option<usize>,
    pub last_play: Option<LastPlay>,
    pub consecutive_passes: usize,
    pub base_score: u32,
    pub multiplier: u32,
    pub highest_bid: u8,
    pub winner: Option<usize>, // Player index
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Action {
    Bid { score: u8 }, // 0, 1, 2, 3
    Play { cards: Vec<Card> },
    Pass,
}

pub const APP_ID: &str = "doudizhu";
pub const APP_REVISION: u32 = 1;

pub fn get_app() -> App {
    App {
        id: APP_ID.to_string(),
        revision: APP_REVISION,
    }
}

impl DouDizhuGame {
    pub fn new(players: Vec<User>) -> Self {
        let player_states = players
            .into_iter()
            .map(|p| PlayerState {
                player: p,
                hand: Vec::new(),
                role: Role::Undecided,
                has_passed: false,
                bid_score: 0,
            })
            .collect();

        DouDizhuGame {
            players: player_states,
            deck: Deck::new_with_jokers(),
            hole_cards: Vec::new(),
            stage: Stage::Setup,
            current_turn: 0,
            landlord_idx: None,
            last_play: None,
            consecutive_passes: 0,
            base_score: 0,
            multiplier: 1,
            highest_bid: 0,
            winner: None,
        }
    }

    fn next_turn(&mut self) {
        self.current_turn = (self.current_turn + 1) % 3;
    }

    fn sort_hand(&mut self, player_idx: usize) {
        self.players[player_idx].hand.sort_by(|a, b| {
            let rank_a = DouDizhuRank::from(a);
            let rank_b = DouDizhuRank::from(b);
            rank_b.cmp(&rank_a) // Descending
        });
    }

    fn deal_cards(&mut self) {
        self.deck = Deck::new_with_jokers();
        self.deck.shuffle();

        self.hole_cards.clear();
        for _ in 0..3 {
            self.hole_cards.push(self.deck.deal().unwrap());
        }

        for i in 0..3 {
            self.players[i].hand.clear();
            for _ in 0..17 {
                if let Some(card) = self.deck.deal() {
                    self.players[i].hand.push(card);
                }
            }
            self.sort_hand(i);
        }
    }
}

impl Game for DouDizhuGame {
    fn meta(&self) -> GameMeta {
        GameMeta {
            app: get_app(),
            description: "Classic Chinese card game for 3 players".to_string(),
        }
    }

    fn start(&mut self) -> UpdateGameState {
        if self.players.len() != 3 {
            // In a real scenario, this might return an error or wait.
        }

        self.deal_cards();

        self.stage = Stage::Bidding;
        self.current_turn = rand::rng().random_range(0..3); // Random start for bidding
        self.highest_bid = 0;
        self.base_score = 0;
        self.consecutive_passes = 0;
        self.multiplier = 1;
        self.landlord_idx = None;
        self.last_play = None;
        self.winner = None;

        for p in &mut self.players {
            p.role = Role::Undecided;
            p.bid_score = 0;
            p.has_passed = false;
        }

        UpdateGameState::snapshot(self.snapshot())
    }

    fn handle_action(&mut self, message: GameMessage) -> UpdateGameState {
        let player_idx_opt = self
            .players
            .iter()
            .position(|p| p.player.id == message.player_id);

        if player_idx_opt.is_none() {
            return UpdateGameState::snapshot(self.snapshot());
        }
        let player_idx = player_idx_opt.unwrap();

        // Must be current turn
        if player_idx != self.current_turn {
            return UpdateGameState::snapshot(self.snapshot());
        }

        let action: Action = match serde_json::from_slice(&message.message.data) {
            Ok(a) => a,
            Err(_) => return UpdateGameState::snapshot(self.snapshot()),
        };

        // Clone message for history before moving out of scope (actually we just need the struct)
        let accepted_message = AcceptedMessage { seq: 0, message };

        match self.stage {
            Stage::Bidding => {
                if let Action::Bid { score } = action {
                    let mut valid_bid = false;

                    if score == 0 {
                        // Pass
                        self.consecutive_passes += 1;
                        self.players[player_idx].has_passed = true;
                        valid_bid = true;
                    } else if score > self.highest_bid && score <= 3 {
                        // Valid raise
                        self.highest_bid = score;
                        self.base_score = score as u32;
                        self.landlord_idx = Some(player_idx);
                        self.players[player_idx].bid_score = score;
                        self.consecutive_passes = 0; // Reset consecutive pass count
                        valid_bid = true;
                    }

                    if valid_bid {
                        // Check if bidding ends
                        // Condition 1: Someone bids 3 -> Instant Landlord
                        // Condition 2: Everyone passed (3 passes) -> Redeal
                        // Condition 3: Bid made, then 2 passes -> Bidder is Landlord

                        let bidding_ended = if self.highest_bid == 3 {
                            true
                        } else if self.highest_bid == 0 && self.consecutive_passes >= 3 {
                            // Redeal
                            self.start();
                            return UpdateGameState::snapshot(self.snapshot());
                        } else if self.highest_bid > 0 && self.consecutive_passes >= 2 {
                            true
                        } else {
                            false
                        };

                        if bidding_ended {
                            if let Some(landlord) = self.landlord_idx {
                                // Give hole cards
                                self.players[landlord].hand.extend(self.hole_cards.clone());
                                self.sort_hand(landlord);
                                self.players[landlord].role = Role::Landlord;
                                for i in 0..3 {
                                    if i != landlord {
                                        self.players[i].role = Role::Peasant;
                                    }
                                }

                                self.stage = Stage::Playing;
                                self.current_turn = landlord;
                                self.consecutive_passes = 0;
                                self.last_play = None;
                            }
                        } else {
                            self.next_turn();
                        }
                    }
                }
            }
            Stage::Playing => {
                match action {
                    Action::Play { cards } => {
                        // 1. Analyze pattern
                        if let Some(pattern) = analyze_pattern(&cards) {
                            // 2. Validate against last play
                            let mut valid = true;
                            if let Some(last) = &self.last_play {
                                if last.player_idx != player_idx {
                                    // Following someone else
                                    if !pattern.beats(&last.pattern) {
                                        valid = false;
                                    }
                                } else {
                                    // It's my turn again (everyone passed), I can play anything valid
                                }
                            }

                            // 3. Verify player has these cards
                            let mut hand_indices = Vec::new();
                            let mut temp_hand = self.players[player_idx].hand.clone();

                            for card in &cards {
                                if let Some(pos) = temp_hand.iter().position(|c| c == card) {
                                    hand_indices.push(pos);
                                    temp_hand.remove(pos); // Remove to handle duplicates correctly if any (though standard deck has unique cards)
                                } else {
                                    valid = false;
                                    break;
                                }
                            }

                            if valid {
                                // Apply update
                                self.players[player_idx].hand = temp_hand;

                                // Update Last Play
                                self.last_play = Some(LastPlay {
                                    player_idx,
                                    cards,
                                    pattern: pattern.clone(),
                                });
                                self.consecutive_passes = 0;

                                // Check Bomb/Rocket multiplier
                                if let Pattern::Bomb(_) = pattern {
                                    self.multiplier *= 2;
                                } else if let Pattern::Rocket = pattern {
                                    self.multiplier *= 2;
                                }

                                // Check Win
                                if self.players[player_idx].hand.is_empty() {
                                    self.stage = Stage::Finished;
                                    self.winner = Some(player_idx);
                                } else {
                                    self.next_turn();
                                }
                            }
                        }
                    }
                    Action::Pass => {
                        // Can only pass if not free turn
                        let can_pass = if let Some(last) = &self.last_play {
                            last.player_idx != player_idx
                        } else {
                            false
                        };

                        if can_pass {
                            self.consecutive_passes += 1;
                            self.next_turn();

                            // If 2 people passed, next player starts new round
                            if self.consecutive_passes >= 2 {
                                self.last_play = None;
                                self.consecutive_passes = 0;
                                // Turn is already next player, who is the winner of previous trick
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        UpdateGameState {
            messages: vec![accepted_message],
            state: self.snapshot(),
        }
    }

    fn snapshot(&self) -> TypedData {
        let json = serde_json::to_vec(self).unwrap();
        TypedData {
            r#type: DataType {
                app: get_app(),
                r#type: "doudizhu_state".to_string(),
            },
            codec: "json".to_string(),
            data: Data(Bytes::from(json)),
        }
    }
}
