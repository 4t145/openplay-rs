use crate::{Action, DouDizhuGame, Stage};
use openplay_basic::user::UserId;
use openplay_poker::Card;

pub struct SimpleBotLogic;

impl SimpleBotLogic {
    pub fn decide(player_id: &UserId, game: &DouDizhuGame) -> Option<Action> {
        // 1. Check if finished
        if let Stage::Finished = game.stage {
            return None;
        }

        // 2. Check turn
        let my_idx = game
            .players
            .iter()
            .position(|p| p.player.id == *player_id)?;

        if my_idx != game.current_turn {
            return None;
        }

        // 3. Logic
        match game.stage {
            Stage::Bidding => {
                // If I have passed already, I shouldn't be here (turn skips me).
                // But just in case logic:
                if game.highest_bid == 0 {
                    // Start bidding
                    Some(Action::Bid { score: 1 })
                } else {
                    // Simple logic: always pass if someone already bid
                    Some(Action::Bid { score: 0 })
                }
            }
            Stage::Playing => {
                let hand = &game.players[my_idx].hand;
                if hand.is_empty() {
                    return Some(Action::Pass);
                }

                let last_play = &game.last_play;

                // If last play is None, or it's my own play (everyone passed back to me)
                let is_free_turn = if let Some(last) = last_play {
                    last.player_idx == my_idx
                } else {
                    true
                };

                if is_free_turn {
                    // Play smallest single
                    // Hand is sorted Descending (High to Low), so smallest is last
                    if let Some(card) = hand.last() {
                        Some(Action::Play { cards: vec![*card] })
                    } else {
                        // Should not happen if check above passes
                        Some(Action::Pass)
                    }
                } else {
                    // Must follow
                    let last = last_play.as_ref().unwrap();
                    match last.pattern {
                        crate::pattern::Pattern::Single(rank_to_beat) => {
                            let mut best_card: Option<Card> = None;
                            // Iterate from smallest (end) to largest (start)
                            for card in hand.iter().rev() {
                                let my_rank = crate::pattern::DouDizhuRank::from(card);
                                if my_rank > rank_to_beat {
                                    best_card = Some(*card);
                                    break;
                                }
                            }

                            if let Some(card) = best_card {
                                Some(Action::Play { cards: vec![card] })
                            } else {
                                Some(Action::Pass)
                            }
                        }
                        _ => {
                            // Cannot handle other patterns yet
                            Some(Action::Pass)
                        }
                    }
                }
            }
            _ => None,
        }
    }
}
