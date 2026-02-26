use crate::{Dragon, Honer, SuitKind, TileFace, UNICODE_START, Wind};

impl TileFace {
    pub fn unicode(&self) -> char {
        unsafe { char::from_u32_unchecked(UNICODE_START as u32 + self.0 as u32) }
    }
}

impl Honer {
    pub const fn unicode(&self) -> char {
        match self {
            Honer::Wind(w) => w.unicode(),
            Honer::Dragon(d) => d.unicode(),
        }
    }
}

impl SuitKind {
    pub const fn unicode_start(&self) -> char {
        match self {
            SuitKind::Dot => '🀙',
            SuitKind::Bamboo => '🀐',
            SuitKind::Character => '🀇',
        }
    }
}

impl Dragon {
    pub const fn unicode(self) -> char {
        match self {
            Dragon::Red => '🀄',
            Dragon::Green => '🀅',
            Dragon::White => '🀆',
        }
    }
}

impl Wind {
    pub const fn unicode(self) -> char {
        match self {
            Wind::East => '🀀',
            Wind::South => '🀁',
            Wind::West => '🀂',
            Wind::North => '🀃',
        }
    }
}
