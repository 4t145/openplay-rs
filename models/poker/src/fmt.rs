use std::fmt::Display;

use crate::{Card, NaturalCard, Rank, Suit};

impl Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Card::NaturalCard(nc) => write!(f, "{}", nc),
            Card::RedJoker => write!(f, "Red Joker"),
            Card::BlackJoker => write!(f, "Black Joker"),
            Card::WildCard => write!(f, "Wild Card"),
        }
    }
}

impl Display for NaturalCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rank_str = match self.rank {
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "10",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
            Rank::Ace => "A",
        };
        let suit_str = match self.suit {
            Suit::Hearts => "♥",
            Suit::Diamonds => "♦",
            Suit::Clubs => "♣",
            Suit::Spades => "♠",
        };
        write!(f, "{}{}", rank_str, suit_str)
    }
}


pub struct Cards<'c>(pub &'c [Card]);
impl Display for Cards<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for card in self.0 {
            write!(f, "[{}] ", card)?;
        }
        Ok(())
    }
}