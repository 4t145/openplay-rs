use std::collections::HashMap;
use std::time::Duration;

use crate::pattern::{analyze_pattern, Pattern};
use bytes::Bytes;
use openplay_basic::data::Data;
use openplay_basic::game::{
    ClientEvent, Game, GameCommand, GameEvent, GameMeta, GameState, GameUpdate, GameViewUpdate, Id,
    SequencedGameUpdate, TimeExpired,
};
use openplay_basic::message::{App, DataType, TypedData};
use openplay_basic::room::{RoomContext, RoomPlayerPosition, RoomView};
use openplay_basic::user::{
    Action as UserAction, ActionData as UserActionData, ActionSource, User,
};
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
    pub version: u32, // Optimistic locking version
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
    #[serde(skip)]
    pub timer_id: Option<Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Action {
    Bid { score: u8 }, // 0, 1, 2, 3
    Play { cards: Vec<Card> },
    Pass,
}

// Local helper for client-side construction, but server uses openplay_basic types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionData {
    pub action: Action,
    pub ref_version: u32,
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
            version: 0,
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
            timer_id: None,
        }
    }

    fn next_turn(&mut self) -> Vec<GameCommand> {
        self.current_turn = (self.current_turn + 1) % 3;
        self.start_turn_timer()
    }

    fn sort_hand(&mut self, player_idx: usize) {
        use crate::pattern::DouDizhuRank;
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

    fn start_turn_timer(&mut self) -> Vec<GameCommand> {
        let mut commands = Vec::new();
        // Cancel existing timer if any
        if let Some(timer_id) = self.timer_id.take() {
            commands.push(GameCommand::CancelTimer {
                id: timer_id,
                duration: Duration::ZERO, // Duration is ignored for cancel usually
            });
        }

        // Create new timer
        let new_timer_id = Id::from(uuid::Uuid::new_v4().to_string());
        self.timer_id = Some(new_timer_id.clone());
        commands.push(GameCommand::CreateTimer {
            id: new_timer_id,
            duration: Duration::from_secs(30),
        });

        commands
    }

    fn handle_user_action(
        &mut self,
        _ctx: &RoomContext,
        action: UserAction,
    ) -> (Vec<ClientEvent>, Vec<GameCommand>) {
        // Extract Player ID
        let player_id = match action.source {
            ActionSource::User(uid) => uid,
            _ => return (vec![], vec![]),
        };

        // Extract Payload
        let game_action_data = match action.data {
            UserActionData::GameAction(data) => data,
            _ => return (vec![], vec![]),
        };

        // Optimistic Locking
        if game_action_data.ref_version != self.version {
            tracing::warn!(
                "Optimistic lock failure: player {:?} sent version {}, current is {}",
                player_id,
                game_action_data.ref_version,
                self.version
            );
            return (vec![], vec![]);
        }

        // Deserialize Action
        let doudizhu_action: Action = match serde_json::from_slice(&game_action_data.message.data.0)
        {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!("Failed to deserialize action: {:?}", e);
                return (vec![], vec![]);
            }
        };

        // Find Player Index
        let player_idx_opt = self.players.iter().position(|p| p.player.id == player_id);

        let player_idx = match player_idx_opt {
            Some(idx) => idx,
            None => {
                tracing::warn!("Player not found: {:?}", player_id);
                return (vec![], vec![]);
            }
        };

        // Must be current turn
        if player_idx != self.current_turn {
            tracing::warn!(
                "Not player's turn: {:?} (current: {})",
                player_id,
                self.current_turn
            );
            return (vec![], vec![]);
        }

        self.process_game_action(player_idx, doudizhu_action)
    }

    fn process_game_action(
        &mut self,
        player_idx: usize,
        action: Action,
    ) -> (Vec<ClientEvent>, Vec<GameCommand>) {
        let mut commands = Vec::new();
        let events = Vec::new();

        // Check if action is valid and update state
        let mut state_changed = false;

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
                        state_changed = true;
                        // Check if bidding ends
                        let bidding_ended = if self.highest_bid == 3 {
                            true
                        } else if self.highest_bid == 0 && self.consecutive_passes >= 3 {
                            // Redeal
                            self.start(); // This resets state
                            return (vec![], self.start_turn_timer());
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
                                commands.extend(self.start_turn_timer());
                            }
                        } else {
                            commands.extend(self.next_turn());
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
                            let mut temp_hand = self.players[player_idx].hand.clone();
                            let mut has_cards = true;

                            for card in &cards {
                                if let Some(pos) = temp_hand.iter().position(|c| c == card) {
                                    temp_hand.remove(pos);
                                } else {
                                    has_cards = false;
                                    break;
                                }
                            }

                            if valid && has_cards {
                                state_changed = true;
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
                                    // Cancel timer if game finished
                                    if let Some(timer_id) = self.timer_id.take() {
                                        commands.push(GameCommand::CancelTimer {
                                            id: timer_id,
                                            duration: Duration::ZERO,
                                        });
                                    }
                                } else {
                                    commands.extend(self.next_turn());
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
                            state_changed = true;
                            self.consecutive_passes += 1;
                            commands.extend(self.next_turn());

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

        if state_changed {
            self.version += 1;
        }

        (events, commands)
    }

    fn handle_timer_expired(
        &mut self,
        _timer: TimeExpired,
    ) -> (Vec<ClientEvent>, Vec<GameCommand>) {
        if let Some(current_timer) = &self.timer_id {
            if current_timer != &_timer.timer_id {
                return (vec![], vec![]);
            }
        } else {
            return (vec![], vec![]);
        }

        let action = match self.stage {
            Stage::Bidding => Action::Bid { score: 0 }, // Pass
            Stage::Playing => Action::Pass,             // Pass
            _ => return (vec![], vec![]),
        };

        if self.stage == Stage::Playing {
            if let Some(last) = &self.last_play {
                if last.player_idx == self.current_turn {
                    if let Some(lowest) = self.players[self.current_turn].hand.last() {
                        return self.process_game_action(
                            self.current_turn,
                            Action::Play {
                                cards: vec![*lowest],
                            },
                        );
                    }
                }
            }
        }

        self.process_game_action(self.current_turn, action)
    }

    pub fn start(&mut self) {
        if self.players.len() != 3 {
            return;
        }

        self.deal_cards();

        self.stage = Stage::Bidding;
        self.current_turn = rand::rng().random_range(0..3);
        self.highest_bid = 0;
        self.base_score = 0;
        self.consecutive_passes = 0;
        self.multiplier = 1;
        self.landlord_idx = None;
        self.last_play = None;
        self.winner = None;
        self.version = 1;

        for p in &mut self.players {
            p.role = Role::Undecided;
            p.bid_score = 0;
            p.has_passed = false;
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

    // Create a masked view for a specific player
    fn masked_snapshot(&self, for_player_idx: Option<usize>) -> TypedData {
        let masked_game = self.clone();

        if let Ok(mut value) = serde_json::to_value(&masked_game) {
            if let Some(players) = value.get_mut("players").and_then(|v| v.as_array_mut()) {
                for (i, p_val) in players.iter_mut().enumerate() {
                    if Some(i) != for_player_idx {
                        if let Some(obj) = p_val.as_object_mut() {
                            obj.insert("hand".to_string(), serde_json::json!([]));
                        }
                    }
                }
            }
            if masked_game.stage == Stage::Setup || masked_game.stage == Stage::Bidding {
                if let Some(obj) = value.as_object_mut() {
                    obj.insert("hole_cards".to_string(), serde_json::json!([]));
                }
            }

            let json = serde_json::to_vec(&value).unwrap();
            TypedData {
                r#type: DataType {
                    app: get_app(),
                    r#type: "doudizhu_state".to_string(),
                },
                codec: "json".to_string(),
                data: Data(Bytes::from(json)),
            }
        } else {
            self.snapshot()
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

    fn handle_action(&mut self, ctx: &RoomContext, update: SequencedGameUpdate) -> GameUpdate {
        // Handle GameStart
        if let GameEvent::GameStart = update.event {
            self.start();
            let start_commands = self.start_turn_timer();
            return self.make_update(ctx, vec![], start_commands);
        }

        let (events, commands) = match update.event {
            GameEvent::Action(action) => self.handle_user_action(ctx, action),
            GameEvent::TimerExpired(timer) => self.handle_timer_expired(timer),
            _ => (vec![], vec![]),
        };

        // If no events and no commands were generated, and state didn't change (version is same),
        // we should not broadcast a view update to avoid infinite loops with bots.
        // However, handle_user_action might return empty vectors on error.
        // We can check if events and commands are empty.
        if events.is_empty() && commands.is_empty() {
            // We can check if version changed, but handle_user_action only increments version on success.
            // If we are here, it means either:
            // 1. Action was rejected (lock failure, invalid move, etc)
            // 2. Timer expired but no action was taken (e.g. wrong timer id)
            // 3. Unknown event

            // In all these cases, the state hasn't changed.
            // Returning an empty update prevents the loop.
            return GameUpdate {
                views: HashMap::new(),
                snapshot: GameState {
                    version: self.version,
                    data: self.snapshot(),
                },
                commands: vec![],
            };
        }

        self.make_update(ctx, events, commands)
    }
}

impl DouDizhuGame {
    fn make_update(
        &self,
        _ctx: &RoomContext,
        events: Vec<ClientEvent>,
        commands: Vec<GameCommand>,
    ) -> GameUpdate {
        let mut views = HashMap::new();

        // 1. Player Views
        for (i, _) in self.players.iter().enumerate() {
            let pos = RoomPlayerPosition::from(i.to_string());
            let view_state = GameState {
                version: self.version,
                data: self.masked_snapshot(Some(i)),
            };

            views.insert(
                RoomView::Position(pos),
                GameViewUpdate {
                    events: events.clone(),
                    new_view: view_state,
                },
            );
        }

        // 2. Observer View (Neutral)
        let observer_view_state = GameState {
            version: self.version,
            data: self.masked_snapshot(None), // See everything masked
        };
        views.insert(
            RoomView::Neutral,
            GameViewUpdate {
                events: events.clone(),
                new_view: observer_view_state,
            },
        );

        GameUpdate {
            views,
            snapshot: GameState {
                version: self.version,
                data: self.snapshot(), // Server full state
            },
            commands,
        }
    }
}
