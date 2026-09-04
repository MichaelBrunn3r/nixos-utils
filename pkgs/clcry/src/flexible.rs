use crate::buffer::Buffer;
use crate::view::{Constraints, Rect, Size, View};

/// Marks a view as eligible to receive spare space in a linear layout.
pub struct Flexible<V> {
    child: V,
    grow: usize,
}

impl<V> Flexible<V> {
    /// Wraps a view with a default grow factor of one.
    #[must_use]
    pub const fn new(child: V) -> Self {
        Self { child, grow: 1 }
    }

    /// Sets the relative share of spare space assigned to this view.
    #[must_use]
    pub const fn grow(mut self, grow: usize) -> Self {
        self.grow = grow;
        self
    }
}

impl<V: View> View for Flexible<V> {
    fn measure(&mut self, constraints: Constraints) -> Size {
        self.child.measure(constraints)
    }

    fn flex_grow(&self) -> usize {
        self.grow
    }

    fn arrange(&mut self, bounds: Rect) {
        self.child.arrange(bounds);
    }

    fn render(&self, buffer: &mut Buffer) {
        self.child.render(buffer);
    }
}
