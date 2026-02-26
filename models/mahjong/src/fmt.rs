use crate::{Num, TileFace};

impl std::fmt::Display for Num {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

pub struct UnicodeTileFace<'a>(&'a TileFace);

impl std::fmt::Display for UnicodeTileFace<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.unicode())
    }
}

impl TileFace {
    pub fn display_unicode(&self) -> UnicodeTileFace<'_> {
        UnicodeTileFace(self)
    }
}


