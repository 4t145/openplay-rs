pub mod fmt;
mod unicode;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Wind {
    East = 0,
    South = 1,
    West = 2,
    North = 3,
}

impl Wind {
    pub const fn from_index(idx: usize) -> Self {
        match idx % 4 {
            0 => Wind::East,
            1 => Wind::South,
            2 => Wind::West,
            3 => Wind::North,
            _ => unreachable!(),
        }
    }
    pub const fn as_index(&self) -> usize {
        match self {
            Wind::East => 0,
            Wind::South => 1,
            Wind::West => 2,
            Wind::North => 3,
        }
    }
    pub fn enumerate() -> <[Self; 4] as IntoIterator>::IntoIter {
        [Wind::East, Wind::South, Wind::West, Wind::North].into_iter()
    }
    pub fn iter_from(self) -> impl Iterator<Item = Wind> + Clone {
        match self {
            Wind::East => [Wind::East, Wind::South, Wind::West, Wind::North].into_iter(),
            Wind::South => [Wind::South, Wind::West, Wind::North, Wind::East].into_iter(),
            Wind::West => [Wind::West, Wind::North, Wind::East, Wind::South].into_iter(),
            Wind::North => [Wind::North, Wind::East, Wind::South, Wind::West].into_iter(),
        }
    }
    pub const fn next(self) -> Self {
        match self {
            Wind::East => Wind::South,
            Wind::South => Wind::West,
            Wind::West => Wind::North,
            Wind::North => Wind::East,
        }
    }
}


#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TileFace(pub(crate) u8);

impl std::fmt::Debug for TileFace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c: char = (*self).into();
        write!(f, "{}", c)
    }
}
impl TileFace {
    pub const fn into_inner(self) -> u8 {
        self.0
    }

    pub const fn const_from_char(c: char) -> Self {
        let face = c as u32 - UNICODE_START as u32;
        TileFace(face as u8)
    }
    pub const fn from_suit(suit: Suit) -> Self {
        let kind = suit.kind;
        let num = suit.num;
        let face = kind.unicode_start() as u32 + num as u32 - 1;
        TileFace(face as u8)
    }
    pub const fn from_honor(honor: Honer) -> Self {
        let face = honor.unicode() as u32 - UNICODE_START as u32;
        TileFace(face as u8)
    }
    pub const fn try_into_suit(self) -> Option<Suit> {
        match self {
            TileFace(16..=24) => Some(Suit {
                kind: SuitKind::Bamboo,
                num: Num::const_from_u8(self.0 - 15),
            }),
            TileFace(7..=15) => Some(Suit {
                kind: SuitKind::Character,
                num: Num::const_from_u8(self.0 - 6),
            }),
            TileFace(25..=33) => Some(Suit {
                kind: SuitKind::Dot,
                num: Num::const_from_u8(self.0 - 24),
            }),
            _ => None,
        }
    }

    pub const fn try_into_honer(self) -> Option<Honer> {
        match self {
            EAST => Some(Honer::Wind(Wind::East)),
            SOUTH => Some(Honer::Wind(Wind::South)),
            WEST => Some(Honer::Wind(Wind::West)),
            NORTH => Some(Honer::Wind(Wind::North)),
            RED => Some(Honer::Dragon(Dragon::Red)),
            GREEN => Some(Honer::Dragon(Dragon::Green)),
            WHITE => Some(Honer::Dragon(Dragon::White)),
            _ => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.is_honor()
            || self
                .try_into_suit()
                .is_some_and(|s| s.num == Num::N1 || s.num == Num::N9)
    }

    pub const fn is_honor(&self) -> bool {
        self.try_into_honer().is_some()
    }

    pub const fn from_honer(honer: Honer) -> Self {
        match honer {
            Honer::Wind(w) => match w {
                Wind::East => EAST,
                Wind::South => SOUTH,
                Wind::West => WEST,
                Wind::North => NORTH,
            },
            Honer::Dragon(d) => match d {
                Dragon::Red => RED,
                Dragon::Green => GREEN,
                Dragon::White => WHITE,
            },
        }
    }
}

impl From<Honer> for TileFace {
    fn from(honer: Honer) -> Self {
        TileFace::from_honor(honer)
    }
}
macro_rules! const_tiles {
    (
        $(
            $name:ident: $face:literal
        )*
    ) => {
        $(
            pub const $name: TileFace = TileFace::const_from_char($face);
        )*
    };
}
const_tiles! {
    B1: '🀐'
    B2: '🀑'
    B3: '🀒'
    B4: '🀓'
    B5: '🀔'
    B6: '🀕'
    B7: '🀖'
    B8: '🀗'
    B9: '🀘'
    C1: '🀇'
    C2: '🀈'
    C3: '🀉'
    C4: '🀊'
    C5: '🀋'
    C6: '🀌'
    C7: '🀍'
    C8: '🀎'
    C9: '🀏'
    D1: '🀙'
    D2: '🀚'
    D3: '🀛'
    D4: '🀜'
    D5: '🀝'
    D6: '🀞'
    D7: '🀟'
    D8: '🀠'
    D9: '🀡'
    EAST: '🀀'
    SOUTH: '🀁'
    WEST: '🀂'
    NORTH: '🀃'
    RED: '🀄'
    GREEN: '🀅'
    WHITE: '🀆'

    PLUM: '🀢'
    ORCHID: '🀣'
    CHRYSANTHEMUM: '🀤'
    BAMBOO: '🀥'

    SPRING: '🀦'
    SUMMER: '🀧'
    AUTUMN: '🀨'
    WINTER: '🀩'

    WILDCARD: '🀪'
    SEASON: '🀫'
}

pub(crate) const UNICODE_START: char = '\u{1F000}';

impl From<char> for TileFace {
    fn from(c: char) -> Self {
        Self::const_from_char(c)
    }
}

impl From<TileFace> for char {
    fn from(val: TileFace) -> Self {
        unsafe { char::from_u32_unchecked(UNICODE_START as u32 + val.0 as u32) }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Num {
    N1 = 1,
    N2 = 2,
    N3 = 3,
    N4 = 4,
    N5 = 5,
    N6 = 6,
    N7 = 7,
    N8 = 8,
    N9 = 9,
}

impl Num {
    pub fn try_from_u8(n: u8) -> Option<Self> {
        match n {
            1 => Some(Num::N1),
            2 => Some(Num::N2),
            3 => Some(Num::N3),
            4 => Some(Num::N4),
            5 => Some(Num::N5),
            6 => Some(Num::N6),
            7 => Some(Num::N7),
            8 => Some(Num::N8),
            9 => Some(Num::N9),
            _ => None,
        }
    }
    pub fn enumerate() -> <[Self; 9] as IntoIterator>::IntoIter {
        [
            Num::N1,
            Num::N2,
            Num::N3,
            Num::N4,
            Num::N5,
            Num::N6,
            Num::N7,
            Num::N8,
            Num::N9,
        ]
        .into_iter()
    }
    pub const fn prev_and_next(self) -> Option<(Num, Num)> {
        match self {
            Num::N1 => None,
            Num::N9 => None,
            _ => {
                let n = self as u8;
                Some((Num::const_from_u8(n - 1), Num::const_from_u8(n + 1)))
            }
        }
    }
    pub const fn next(self) -> Option<Num> {
        match self {
            Num::N9 => None,
            _ => {
                let n = self as u8;
                Some(Num::const_from_u8(n + 1))
            }
        }
    }
    pub const fn next_two(self) -> Option<(Num, Num)> {
        match self {
            Num::N8 | Num::N9 => Some((Num::N1, Num::N2)),
            _ => {
                let n = self as u8;
                Some((Num::const_from_u8(n + 1), Num::const_from_u8(n + 2)))
            }
        }
    }
    pub const fn prev(self) -> Option<Num> {
        match self {
            Num::N1 => None,
            _ => {
                let n = self as u8;
                Some(Num::const_from_u8(n - 1))
            }
        }
    }
    pub const fn prev_two(self) -> Option<(Num, Num)> {
        match self {
            Num::N1 | Num::N2 => None,
            _ => {
                let n = self as u8;
                Some((Num::const_from_u8(n - 1), Num::const_from_u8(n - 2)))
            }
        }
    }
    pub const fn const_from_u8(n: u8) -> Self {
        match n {
            1 => Num::N1,
            2 => Num::N2,
            3 => Num::N3,
            4 => Num::N4,
            5 => Num::N5,
            6 => Num::N6,
            7 => Num::N7,
            8 => Num::N8,
            9 => Num::N9,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Honer {
    Wind(Wind),
    Dragon(Dragon),
}

impl Honer {
    pub fn enumerate() -> <[Self; 7] as IntoIterator>::IntoIter {
        [
            Honer::Wind(Wind::East),
            Honer::Wind(Wind::South),
            Honer::Wind(Wind::West),
            Honer::Wind(Wind::North),
            Honer::Dragon(Dragon::Red),
            Honer::Dragon(Dragon::Green),
            Honer::Dragon(Dragon::White),
        ]
        .into_iter()
    }

}



#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Suit {
    pub kind: SuitKind,
    pub num: Num,
}

impl From<Suit> for TileFace {
    fn from(suit: Suit) -> Self {
        TileFace::from_suit(suit)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SuitKind {
    Bamboo,
    Character,
    Dot,
}

impl SuitKind {
    pub fn enumerate() -> <[Self; 3] as IntoIterator>::IntoIter {
        [SuitKind::Dot, SuitKind::Bamboo, SuitKind::Character].into_iter()
    }

}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Dragon {
    Red,
    Green,
    White,
}

impl Dragon {
    pub fn enumerate() -> <[Self; 3] as IntoIterator>::IntoIter {
        [Dragon::Red, Dragon::Green, Dragon::White].into_iter()
    }

}
