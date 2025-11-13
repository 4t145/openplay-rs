use openplay_basic::player::Player;
use openplay_poker::{Card, Deck};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStage {
    Setup,    // 准备阶段
    PreFlop,  // 翻牌前
    Flop,     // 翻牌
    Turn,     // 转牌
    River,    // 河牌
    Showdown, // 摊牌
    Finished, // 结束
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BettingAction {
    Fold,       // 弃牌
    Check,      // 过牌
    Call,       // 跟注
    Bet(u64),   // 下注
    Raise(u64), // 加注
    AllIn,      // 全下
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerState {
    pub player: Player,
    pub chips: u64,              // 筹码
    pub hand: Vec<Card>,         // 手牌
    pub current_bet: u64,        // 当前下注
    pub total_bet: u64,          // 本轮总下注
    pub is_active: bool,         // 是否还在游戏中
    pub is_all_in: bool,         // 是否全下
}

#[derive(Debug, Clone)]
pub struct HoldemGame {
    pub players: Vec<PlayerState>,
    pub deck: Deck,
    pub community_cards: Vec<Card>, // 公共牌
    pub pot: u64,                   // 主池
    pub side_pots: Vec<SidePot>,    // 边池
    pub current_stage: GameStage,
    pub dealer_position: usize,     // 庄家位置
    pub current_player: usize,      // 当前行动玩家
    pub small_blind: u64,
    pub big_blind: u64,
    pub current_bet: u64,           // 当前最高下注
    pub min_raise: u64,             // 最小加注额
}

#[derive(Debug, Clone)]
pub struct SidePot {
    pub amount: u64,
    pub eligible_players: Vec<usize>, // 有资格竞争这个边池的玩家索引
}

#[derive(Debug, Clone)]
pub enum GameEvent {
    GameStarted {
        players: Vec<Player>,
        dealer_position: usize,
    },
    BlindsPosted {
        small_blind_player: usize,
        small_blind_amount: u64,
        big_blind_player: usize,
        big_blind_amount: u64,
    },
    CardsDealt {
        player: usize,
        cards: Vec<Card>,
    },
    StageChanged {
        stage: GameStage,
    },
    CommunityCardsRevealed {
        cards: Vec<Card>,
    },
    PlayerAction {
        player: usize,
        action: BettingAction,
    },
    PotUpdated {
        pot: u64,
    },
    PlayerWon {
        player: usize,
        amount: u64,
        hand_rank: Option<HandRank>,
    },
    GameEnded,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandRank {
    HighCard(Vec<u8>),
    OnePair(u8, Vec<u8>),
    TwoPair(u8, u8, u8),
    ThreeOfAKind(u8, Vec<u8>),
    Straight(u8),
    Flush(Vec<u8>),
    FullHouse(u8, u8),
    FourOfAKind(u8, u8),
    StraightFlush(u8),
    RoyalFlush,
}

pub type GameResult<T> = Result<T, GameError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameError {
    NotEnoughPlayers,
    InvalidPlayerIndex,
    InvalidAction(String),
    NotPlayerTurn,
    InsufficientChips,
    GameNotInProgress,
    InvalidBetAmount,
}

impl HoldemGame {
    /// 创建新游戏
    pub fn new(players: Vec<Player>, starting_chips: u64, small_blind: u64, big_blind: u64) -> GameResult<Self> {
        if players.len() < 2 {
            return Err(GameError::NotEnoughPlayers);
        }

        let player_states = players.into_iter().map(|player| PlayerState {
            player,
            chips: starting_chips,
            hand: Vec::new(),
            current_bet: 0,
            total_bet: 0,
            is_active: true,
            is_all_in: false,
        }).collect();

        Ok(HoldemGame {
            players: player_states,
            deck: Deck::new(),
            community_cards: Vec::new(),
            pot: 0,
            side_pots: Vec::new(),
            current_stage: GameStage::Setup,
            dealer_position: 0,
            current_player: 0,
            small_blind,
            big_blind,
            current_bet: 0,
            min_raise: big_blind,
        })
    }

    /// 开始新一轮
    pub fn start_round(&mut self) -> Vec<GameEvent> {
        let mut events = Vec::new();

        // 重置状态
        self.deck = Deck::new();
        self.deck.shuffle();
        self.community_cards.clear();
        self.pot = 0;
        self.side_pots.clear();
        self.current_bet = 0;
        self.min_raise = self.big_blind;

        for player in &mut self.players {
            player.hand.clear();
            player.current_bet = 0;
            player.total_bet = 0;
            player.is_active = player.chips > 0;
            player.is_all_in = false;
        }

        events.push(GameEvent::GameStarted {
            players: self.players.iter().map(|p| p.player.clone()).collect(),
            dealer_position: self.dealer_position,
        });

        // 收取盲注
        self.post_blinds(&mut events);

        // 发手牌
        self.deal_hole_cards(&mut events);

        // 进入翻牌前阶段
        self.current_stage = GameStage::PreFlop;
        events.push(GameEvent::StageChanged { stage: GameStage::PreFlop });

        // 设置第一个行动玩家 (大盲注后一位)
        self.current_player = self.next_active_player((self.dealer_position + 3) % self.players.len());

        events
    }

    /// 收取盲注
    fn post_blinds(&mut self, events: &mut Vec<GameEvent>) {
        let sb_pos = (self.dealer_position + 1) % self.players.len();
        let bb_pos = (self.dealer_position + 2) % self.players.len();

        // 小盲注
        let sb_amount = self.small_blind.min(self.players[sb_pos].chips);
        self.players[sb_pos].chips -= sb_amount;
        self.players[sb_pos].current_bet = sb_amount;
        self.players[sb_pos].total_bet = sb_amount;
        self.pot += sb_amount;

        // 大盲注
        let bb_amount = self.big_blind.min(self.players[bb_pos].chips);
        self.players[bb_pos].chips -= bb_amount;
        self.players[bb_pos].current_bet = bb_amount;
        self.players[bb_pos].total_bet = bb_amount;
        self.pot += bb_amount;
        self.current_bet = bb_amount;

        events.push(GameEvent::BlindsPosted {
            small_blind_player: sb_pos,
            small_blind_amount: sb_amount,
            big_blind_player: bb_pos,
            big_blind_amount: bb_amount,
        });
    }

    /// 发手牌
    fn deal_hole_cards(&mut self, events: &mut Vec<GameEvent>) {
        for i in 0..self.players.len() {
            if self.players[i].is_active {
                let card1 = self.deck.deal().unwrap();
                let card2 = self.deck.deal().unwrap();
                self.players[i].hand = vec![card1, card2];
                events.push(GameEvent::CardsDealt {
                    player: i,
                    cards: vec![card1, card2],
                });
            }
        }
    }

    /// 玩家行动
    pub fn player_action(&mut self, player_idx: usize, action: BettingAction) -> GameResult<Vec<GameEvent>> {
        if player_idx != self.current_player {
            return Err(GameError::NotPlayerTurn);
        }

        if !self.players[player_idx].is_active || self.players[player_idx].is_all_in {
            return Err(GameError::InvalidAction("Player is not active".to_string()));
        }

        let mut events = Vec::new();

        match action {
            BettingAction::Fold => {
                self.players[player_idx].is_active = false;
                events.push(GameEvent::PlayerAction { player: player_idx, action });
            }
            BettingAction::Check => {
                if self.players[player_idx].current_bet < self.current_bet {
                    return Err(GameError::InvalidAction("Cannot check, must call or raise".to_string()));
                }
                events.push(GameEvent::PlayerAction { player: player_idx, action });
            }
            BettingAction::Call => {
                let call_amount = self.current_bet - self.players[player_idx].current_bet;
                let actual_amount = call_amount.min(self.players[player_idx].chips);
                
                self.players[player_idx].chips -= actual_amount;
                self.players[player_idx].current_bet += actual_amount;
                self.players[player_idx].total_bet += actual_amount;
                self.pot += actual_amount;

                if self.players[player_idx].chips == 0 {
                    self.players[player_idx].is_all_in = true;
                }

                events.push(GameEvent::PlayerAction { player: player_idx, action });
                events.push(GameEvent::PotUpdated { pot: self.pot });
            }
            BettingAction::Bet(amount) => {
                if self.current_bet > 0 {
                    return Err(GameError::InvalidAction("Cannot bet, must raise".to_string()));
                }
                if amount < self.big_blind {
                    return Err(GameError::InvalidBetAmount);
                }
                if amount > self.players[player_idx].chips {
                    return Err(GameError::InsufficientChips);
                }

                self.players[player_idx].chips -= amount;
                self.players[player_idx].current_bet = amount;
                self.players[player_idx].total_bet += amount;
                self.pot += amount;
                self.current_bet = amount;
                self.min_raise = amount;

                if self.players[player_idx].chips == 0 {
                    self.players[player_idx].is_all_in = true;
                }

                events.push(GameEvent::PlayerAction { player: player_idx, action });
                events.push(GameEvent::PotUpdated { pot: self.pot });
            }
            BettingAction::Raise(amount) => {
                if amount < self.current_bet + self.min_raise {
                    return Err(GameError::InvalidBetAmount);
                }
                if amount > self.players[player_idx].chips + self.players[player_idx].current_bet {
                    return Err(GameError::InsufficientChips);
                }

                let total_amount = amount - self.players[player_idx].current_bet;
                self.players[player_idx].chips -= total_amount;
                self.players[player_idx].current_bet = amount;
                self.players[player_idx].total_bet += total_amount;
                self.pot += total_amount;
                
                let raise_amount = amount - self.current_bet;
                self.min_raise = raise_amount;
                self.current_bet = amount;

                if self.players[player_idx].chips == 0 {
                    self.players[player_idx].is_all_in = true;
                }

                events.push(GameEvent::PlayerAction { player: player_idx, action });
                events.push(GameEvent::PotUpdated { pot: self.pot });
            }
            BettingAction::AllIn => {
                let all_in_amount = self.players[player_idx].chips;
                self.players[player_idx].chips = 0;
                self.players[player_idx].current_bet += all_in_amount;
                self.players[player_idx].total_bet += all_in_amount;
                self.players[player_idx].is_all_in = true;
                self.pot += all_in_amount;

                if self.players[player_idx].current_bet > self.current_bet {
                    let raise_amount = self.players[player_idx].current_bet - self.current_bet;
                    self.min_raise = raise_amount;
                    self.current_bet = self.players[player_idx].current_bet;
                }

                events.push(GameEvent::PlayerAction { player: player_idx, action });
                events.push(GameEvent::PotUpdated { pot: self.pot });
            }
        }

        // 移动到下一个玩家
        self.current_player = self.next_active_player(player_idx + 1);

        // 检查是否需要进入下一阶段
        if self.is_betting_round_complete() {
            self.advance_stage(&mut events);
        }

        Ok(events)
    }

    /// 找到下一个活跃玩家
    fn next_active_player(&self, start: usize) -> usize {
        let mut idx = start % self.players.len();
        for _ in 0..self.players.len() {
            if self.players[idx].is_active && !self.players[idx].is_all_in {
                return idx;
            }
            idx = (idx + 1) % self.players.len();
        }
        idx
    }

    /// 检查本轮下注是否完成
    fn is_betting_round_complete(&self) -> bool {
        let active_players: Vec<_> = self.players.iter()
            .filter(|p| p.is_active && !p.is_all_in)
            .collect();

        if active_players.is_empty() {
            return true;
        }

        if active_players.len() == 1 {
            return true;
        }

        // 所有活跃玩家的当前下注都相等
        active_players.iter().all(|p| p.current_bet == self.current_bet)
    }

    /// 进入下一阶段
    fn advance_stage(&mut self, events: &mut Vec<GameEvent>) {
        // 重置当前下注
        for player in &mut self.players {
            player.current_bet = 0;
        }
        self.current_bet = 0;

        match self.current_stage {
            GameStage::PreFlop => {
                // 发翻牌
                self.deck.deal(); // 烧牌
                let flop = vec![
                    self.deck.deal().unwrap(),
                    self.deck.deal().unwrap(),
                    self.deck.deal().unwrap(),
                ];
                self.community_cards.extend_from_slice(&flop);
                self.current_stage = GameStage::Flop;
                
                events.push(GameEvent::StageChanged { stage: GameStage::Flop });
                events.push(GameEvent::CommunityCardsRevealed { cards: flop });
                
                self.current_player = self.next_active_player((self.dealer_position + 1) % self.players.len());
            }
            GameStage::Flop => {
                // 发转牌
                self.deck.deal(); // 烧牌
                let turn = self.deck.deal().unwrap();
                self.community_cards.push(turn);
                self.current_stage = GameStage::Turn;
                
                events.push(GameEvent::StageChanged { stage: GameStage::Turn });
                events.push(GameEvent::CommunityCardsRevealed { cards: vec![turn] });
                
                self.current_player = self.next_active_player((self.dealer_position + 1) % self.players.len());
            }
            GameStage::Turn => {
                // 发河牌
                self.deck.deal(); // 烧牌
                let river = self.deck.deal().unwrap();
                self.community_cards.push(river);
                self.current_stage = GameStage::River;
                
                events.push(GameEvent::StageChanged { stage: GameStage::River });
                events.push(GameEvent::CommunityCardsRevealed { cards: vec![river] });
                
                self.current_player = self.next_active_player((self.dealer_position + 1) % self.players.len());
            }
            GameStage::River => {
                // 进入摊牌
                self.current_stage = GameStage::Showdown;
                events.push(GameEvent::StageChanged { stage: GameStage::Showdown });
                self.showdown(events);
            }
            _ => {}
        }
    }

    /// 摊牌并确定赢家
    fn showdown(&mut self, events: &mut Vec<GameEvent>) {
        // 简化版: 暂时随机选择赢家
        // TODO: 实现真正的牌型比较
        let active_players: Vec<usize> = self.players.iter()
            .enumerate()
            .filter(|(_, p)| p.is_active)
            .map(|(i, _)| i)
            .collect();

        if let Some(&winner) = active_players.first() {
            let win_amount = self.pot;
            self.players[winner].chips += win_amount;
            
            events.push(GameEvent::PlayerWon {
                player: winner,
                amount: win_amount,
                hand_rank: None, // TODO: 计算实际牌型
            });
        }

        self.current_stage = GameStage::Finished;
        events.push(GameEvent::GameEnded);
        
        // 移动庄家位置
        self.dealer_position = (self.dealer_position + 1) % self.players.len();
    }

    /// 获取当前游戏状态
    pub fn get_stage(&self) -> GameStage {
        self.current_stage
    }

    /// 获取当前行动玩家
    pub fn get_current_player(&self) -> usize {
        self.current_player
    }

    /// 获取玩家信息
    pub fn get_player_state(&self, player_idx: usize) -> Option<&PlayerState> {
        self.players.get(player_idx)
    }

    /// 获取公共牌
    pub fn get_community_cards(&self) -> &[Card] {
        &self.community_cards
    }

    /// 获取底池
    pub fn get_pot(&self) -> u64 {
        self.pot
    }

    /// 检查玩家可执行的动作
    pub fn get_valid_actions(&self, player_idx: usize) -> Vec<BettingAction> {
        if player_idx != self.current_player {
            return Vec::new();
        }

        let player = &self.players[player_idx];
        if !player.is_active || player.is_all_in {
            return Vec::new();
        }

        let mut actions = vec![BettingAction::Fold];

        let call_amount = self.current_bet - player.current_bet;

        if call_amount == 0 {
            actions.push(BettingAction::Check);
        }

        if call_amount > 0 && call_amount <= player.chips {
            actions.push(BettingAction::Call);
        }

        if self.current_bet == 0 && player.chips >= self.big_blind {
            actions.push(BettingAction::Bet(self.big_blind));
        }

        if player.chips + player.current_bet > self.current_bet + self.min_raise {
            actions.push(BettingAction::Raise(self.current_bet + self.min_raise));
        }

        if player.chips > 0 {
            actions.push(BettingAction::AllIn);
        }

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    fn create_test_player(id: u8, nickname: &str) -> Player {
        Player {
            id: openplay_basic::player::PlayerId::from(Bytes::from(vec![id])),
            nickname: nickname.to_string(),
            avatar_url: None,
            is_bot: false,
        }
    }

    #[test]
    fn test_game_creation() {
        let players = vec![
            create_test_player(1, "Alice"),
            create_test_player(2, "Bob"),
            create_test_player(3, "Charlie"),
        ];

        let game = HoldemGame::new(players, 1000, 10, 20);
        assert!(game.is_ok());

        let game = game.unwrap();
        assert_eq!(game.players.len(), 3);
        assert_eq!(game.small_blind, 10);
        assert_eq!(game.big_blind, 20);
    }

    #[test]
    fn test_not_enough_players() {
        let players = vec![create_test_player(1, "Alice")];
        let game = HoldemGame::new(players, 1000, 10, 20);
        assert!(matches!(game, Err(GameError::NotEnoughPlayers)));
    }

    #[test]
    fn test_start_round() {
        let players = vec![
            create_test_player(1, "Alice"),
            create_test_player(2, "Bob"),
            create_test_player(3, "Charlie"),
        ];

        let mut game = HoldemGame::new(players, 1000, 10, 20).unwrap();
        let events = game.start_round();

        // 检查事件
        assert!(!events.is_empty());
        
        // 检查盲注已收取
        assert!(game.pot > 0);
        
        // 检查手牌已发
        assert_eq!(game.players[0].hand.len(), 2);
        assert_eq!(game.players[1].hand.len(), 2);
        assert_eq!(game.players[2].hand.len(), 2);
        
        // 检查阶段
        assert_eq!(game.current_stage, GameStage::PreFlop);
    }

    #[test]
    fn test_full_game_flow() {
        let players = vec![
            create_test_player(1, "Alice"),
            create_test_player(2, "Bob"),
        ];

        let mut game = HoldemGame::new(players, 1000, 10, 20).unwrap();
        game.start_round();

        println!("=== Game Started ===");
        println!("Dealer position: {}", game.dealer_position);
        println!("Current stage: {:?}", game.current_stage);
        println!("Current player: {}", game.current_player);
        println!("Pot: {}", game.pot);
        println!("Current bet: {}", game.current_bet);

        // PreFlop - Player 0 (after big blind) acts first
        let current = game.current_player;
        println!("\n=== PreFlop ===");
        println!("Player {} to act", current);
        println!("Valid actions: {:?}", game.get_valid_actions(current));
        
        // Player calls
        let result = game.player_action(current, BettingAction::Call);
        assert!(result.is_ok());
        println!("Player {} calls", current);
        println!("Pot: {}", game.pot);

        // Check if moved to next stage or next player
        if game.current_stage == GameStage::Flop {
            println!("\n=== Flop ===");
            println!("Community cards: {} cards", game.community_cards.len());
            println!("Current player: {}", game.current_player);

            // Both players check
            let current = game.current_player;
            let _ = game.player_action(current, BettingAction::Check);
            println!("Player {} checks", current);

            if game.current_stage == GameStage::Flop {
                let current = game.current_player;
                let _ = game.player_action(current, BettingAction::Check);
                println!("Player {} checks", current);
            }
        }

        if game.current_stage == GameStage::Turn {
            println!("\n=== Turn ===");
            println!("Community cards: {} cards", game.community_cards.len());
            
            let current = game.current_player;
            let _ = game.player_action(current, BettingAction::Check);
            
            if game.current_stage == GameStage::Turn {
                let current = game.current_player;
                let _ = game.player_action(current, BettingAction::Check);
            }
        }

        if game.current_stage == GameStage::River {
            println!("\n=== River ===");
            println!("Community cards: {} cards", game.community_cards.len());
            
            let current = game.current_player;
            let _ = game.player_action(current, BettingAction::Check);
            
            if game.current_stage == GameStage::River {
                let current = game.current_player;
                let _ = game.player_action(current, BettingAction::Check);
            }
        }

        println!("\n=== Game End ===");
        println!("Final stage: {:?}", game.current_stage);
        println!("Player 0 chips: {}", game.players[0].chips);
        println!("Player 1 chips: {}", game.players[1].chips);
    }

    #[test]
    fn test_player_fold() {
        let players = vec![
            create_test_player(1, "Alice"),
            create_test_player(2, "Bob"),
        ];

        let mut game = HoldemGame::new(players, 1000, 10, 20).unwrap();
        game.start_round();

        let current = game.current_player;
        let result = game.player_action(current, BettingAction::Fold);
        assert!(result.is_ok());
        assert!(!game.players[current].is_active);
    }

    #[test]
    fn test_invalid_action_not_turn() {
        let players = vec![
            create_test_player(1, "Alice"),
            create_test_player(2, "Bob"),
        ];

        let mut game = HoldemGame::new(players, 1000, 10, 20).unwrap();
        game.start_round();

        let wrong_player = (game.current_player + 1) % game.players.len();
        let result = game.player_action(wrong_player, BettingAction::Check);
        assert!(matches!(result, Err(GameError::NotPlayerTurn)));
    }

    #[test]
    fn test_betting_and_raising() {
        let players = vec![
            create_test_player(1, "Alice"),
            create_test_player(2, "Bob"),
            create_test_player(3, "Charlie"),
        ];

        let mut game = HoldemGame::new(players, 1000, 10, 20).unwrap();
        game.start_round();

        println!("Initial pot: {}", game.pot);
        println!("Current player: {}", game.current_player);

        // Player raises
        let current = game.current_player;
        let result = game.player_action(current, BettingAction::Raise(40));
        assert!(result.is_ok());
        assert_eq!(game.current_bet, 40);
        
        println!("After raise - pot: {}, current bet: {}", game.pot, game.current_bet);
    }
}
