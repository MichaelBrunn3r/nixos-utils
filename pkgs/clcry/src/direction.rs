use crate::view::{Constraints, Rect, Size};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    Row,
    Column,
}

impl Direction {
    #[must_use]
    pub const fn size(self, parallel: usize, perpendicular: usize) -> Size {
        match self {
            Self::Row => Size::new(parallel, perpendicular),
            Self::Column => Size::new(perpendicular, parallel),
        }
    }

    #[must_use]
    pub const fn constraints(self, parallel: usize, perpendicular: usize) -> Constraints {
        match self {
            Self::Row => Constraints::at_most(parallel, perpendicular),
            Self::Column => Constraints::at_most(perpendicular, parallel),
        }
    }

    #[must_use]
    pub const fn rect(self, parallel: usize, perpendicular: usize, size: Size) -> Rect {
        match self {
            Self::Row => Rect::new(parallel, perpendicular, size.width, size.height),
            Self::Column => Rect::new(perpendicular, parallel, size.width, size.height),
        }
    }
}
