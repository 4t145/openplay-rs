use crate::Card;

impl Card {
    pub fn to_unicode(&self) -> char {
        match self {
            Card::NaturalCard(nc) => {
                let base = match nc.suit {
                    crate::Suit::Hearts => 0x1F0B0,
                    crate::Suit::Diamonds => 0x1F0C0,
                    crate::Suit::Clubs => 0x1F0D0,
                    crate::Suit::Spades => 0x1F0A0,
                };
                let rank_offset = match nc.rank {
                    crate::Rank::Ace => 0x1,
                    crate::Rank::Two => 0x2,
                    crate::Rank::Three => 0x3,
                    crate::Rank::Four => 0x4,
                    crate::Rank::Five => 0x5,
                    crate::Rank::Six => 0x6,
                    crate::Rank::Seven => 0x7,
                    crate::Rank::Eight => 0x8,
                    crate::Rank::Nine => 0x9,
                    crate::Rank::Ten => 0xA,
                    crate::Rank::Jack => 0xB,
                    crate::Rank::Queen => 0xD,
                    crate::Rank::King => 0xE,
                };
                std::char::from_u32(base + rank_offset).unwrap_or('�')
            }
            Card::RedJoker => '🃏',
            Card::BlackJoker => '🃏',
            Card::WildCard => '🃟',
        }
    }
}
