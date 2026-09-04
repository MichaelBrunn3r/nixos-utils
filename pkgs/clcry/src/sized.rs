use crate::buffer::Buffer;
use crate::view::{Constraints, Rect, Size, View};

/// Applies explicit size constraints to a view.
pub struct Sized {
    child: Box<dyn View>,
    constraints: Constraints,
}

impl Sized {
    /// Wraps a view without adding size constraints.
    #[must_use]
    pub fn new(child: impl View + 'static) -> Self {
        Self {
            child: Box::new(child),
            constraints: Constraints::at_most(usize::MAX, usize::MAX),
        }
    }

    /// Sets the exact width.
    #[must_use]
    pub const fn width(mut self, width: usize) -> Self {
        self.constraints = self.constraints.width(width);
        self
    }

    /// Sets the exact height.
    #[must_use]
    pub const fn height(mut self, height: usize) -> Self {
        self.constraints = self.constraints.height(height);
        self
    }

    /// Sets the minimum width.
    #[must_use]
    pub const fn min_width(mut self, width: usize) -> Self {
        self.constraints = self.constraints.min_width(width);
        self
    }

    /// Sets the minimum height.
    #[must_use]
    pub const fn min_height(mut self, height: usize) -> Self {
        self.constraints = self.constraints.min_height(height);
        self
    }

    /// Sets the maximum width.
    #[must_use]
    pub const fn max_width(mut self, width: usize) -> Self {
        self.constraints = self.constraints.max_width(width);
        self
    }

    /// Sets the maximum height.
    #[must_use]
    pub const fn max_height(mut self, height: usize) -> Self {
        self.constraints = self.constraints.max_height(height);
        self
    }
}

impl View for Sized {
    fn measure(&mut self, constraints: Constraints) -> Size {
        let effective = constraints.intersect(self.constraints);
        effective.clamp(self.child.measure(effective))
    }

    fn arrange(&mut self, bounds: Rect) {
        self.child.arrange(bounds);
    }

    fn render(&self, buffer: &mut Buffer) {
        self.child.render(buffer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Span;

    #[test]
    fn applies_exact_minimum_and_maximum_constraints() {
        let parent = Constraints::at_most(20, 20);

        assert_eq!(
            Sized::new(Span::new("hello")).width(10).measure(parent),
            Size::new(10, 1)
        );
        assert_eq!(
            Sized::new(Span::new("hello")).min_width(8).measure(parent),
            Size::new(8, 1)
        );
        assert_eq!(
            Sized::new(Span::new("hello")).max_width(3).measure(parent),
            Size::new(3, 1)
        );
    }

    #[test]
    fn local_constraints_are_clamped_to_parent_constraints() {
        let size = Sized::new(Span::new("hello"))
            .width(20)
            .measure(Constraints::at_most(8, 1));

        assert_eq!(size, Size::new(8, 1));
    }
}
