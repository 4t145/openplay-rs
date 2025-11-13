use rand::rng;
use rand::seq::SliceRandom;

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
    pub fn two_to_ace() -> [Rank; 13] {
        [
            Rank::Two,
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
            Rank::Ace,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NaturalCard {
    pub suit: Suit,
    pub rank: Rank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Card {
    NaturalCard(NaturalCard),
    RedJoker,
    BlackJoker,
    WildCard,
}

impl Card {
    pub fn is_natural(&self) -> bool {
        matches!(self, Card::NaturalCard(_))
    }
    pub fn unwrap_natural(self) -> NaturalCard {
        match self {
            Card::NaturalCard(nc) => nc,
            _ => panic!("Called unwrap_natural on a non-natural card"),
        }
    }
    pub fn new_natural(suit: Suit, rank: Rank) -> Self {
        Card::NaturalCard(NaturalCard { suit, rank })
    }
}

// 一副扑克牌
#[derive(Debug, Clone)]
pub struct Deck {
    cards: Vec<Card>,
}

impl Deck {
    pub fn new() -> Self {
        let mut cards = Vec::new();

        // 创建52张牌
        for &suit in &[Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades] {
            for rank in Rank::two_to_ace() {
                cards.push(Card::new_natural(suit, rank));
            }
        }

        Deck { cards }
    }
    pub fn new_with_jokers() -> Self {
        let mut deck = Deck::new();
        deck.cards.push(Card::RedJoker);
        deck.cards.push(Card::BlackJoker);
        deck
    }
    pub fn new_with_joker_and_wildcard() -> Self {
        let mut deck = Deck::new_with_jokers();
        deck.cards.push(Card::WildCard);
        deck
    }
    pub fn shuffle(&mut self) {
        let mut rng = rng();
        self.cards.shuffle(&mut rng);
    }

    pub fn deal(&mut self) -> Option<Card> {
        self.cards.pop()
    }

    
}

pub mod utils;
pub mod unicode;