use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;

// 扑克牌花色和点数
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Rank {
    Two = 2,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

impl Rank {
    普pub fn al

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NaturalCard {
    pub suit: Suit,
    pub rank: Rank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Card {
    NaturalCard(NaturalCard),
    BigJoker,
    SmallJoker,
    WildCard,
}

// 一副扑克牌
pub struct Deck {
    cards: Vec<Card>,
}

impl Deck {
    pub fn new() -> Self {
        let mut cards = Vec::new();

        // 创建52张牌
        for &suit in &[Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades] {
            for rank in 2..=14 {
                let rank = match rank {
                    2 => Rank::Two,
                    3 => Rank::Three,
                    // ... 其他点数
                    14 => Rank::Ace,
                    _ => continue,
                };
                cards.push(NaturalCard { suit, rank });
            }
        }

        Deck { cards }
    }

    pub fn shuffle(&mut self) {
        let mut rng = thread_rng();
        self.cards.shuffle(&mut rng);
    }

    pub fn deal(&mut self) -> Option<NaturalCard> {
        self.cards.pop()
    }
}

// 玩家
pub struct Player {
    pub chips: u32,
    pub hand: [Option<NaturalCard>; 2],
    pub is_active: bool,
}

// 游戏状态
pub struct TexasHoldem {
    pub deck: Deck,
    pub players: Vec<Player>,
    pub community_cards: Vec<NaturalCard>,
    pub pot: u32,
    pub current_bet: u32,
}

impl TexasHoldem {
    pub fn new(num_players: usize) -> Self {
        let mut deck = Deck::new();
        deck.shuffle();

        let players = (0..num_players)
            .map(|_| Player {
                chips: 1000, // 初始筹码
                hand: [None, None],
                is_active: true,
            })
            .collect();

        TexasHoldem {
            deck,
            players,
            community_cards: Vec::new(),
            pot: 0,
            current_bet: 0,
        }
    }

    pub fn deal_hole_cards(&mut self) {
        for player in &mut self.players {
            if player.is_active {
                player.hand[0] = self.deck.deal();
                player.hand[1] = self.deck.deal();
            }
        }
    }

    pub fn deal_flop(&mut self) {
        // 德州扑克发翻牌前要烧一张牌
        let _burn = self.deck.deal();
        for _ in 0..3 {
            if let Some(card) = self.deck.deal() {
                self.community_cards.push(card);
            }
        }
    }

    pub fn deal_turn(&mut self) {
        let _burn = self.deck.deal();
        if let Some(card) = self.deck.deal() {
            self.community_cards.push(card);
        }
    }

    pub fn deal_river(&mut self) {
        let _burn = self.deck.deal();
        if let Some(card) = self.deck.deal() {
            self.community_cards.push(card);
        }
    }
}

// 牌型判断函数（需要实现）
pub fn evaluate_hand(hole_cards: &[NaturalCard], community_cards: &[NaturalCard]) -> HandRank {
    // 合并所有牌并找出最好的5张牌组合
    // 这里需要实现完整的牌型判断逻辑
    HandRank::HighCard(Rank::Ace) // 示例
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandRank {
    HighCard(Rank),
    OnePair(Rank),
    TwoPair(Rank, Rank),
    ThreeOfAKind(Rank),
    Straight(Rank),
    Flush(Rank),
    FullHouse(Rank, Rank),
    FourOfAKind(Rank),
    StraightFlush(Rank),
    RoyalFlush,
}
