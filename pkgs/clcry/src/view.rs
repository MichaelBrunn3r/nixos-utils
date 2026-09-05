use crate::buffer::Buffer;
use crate::direction::Direction;
use crate::style::ViewStyle;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// A measured view size.
pub struct Size {
    /// The width in cells.
    pub width: usize,
    /// The height in cells.
    pub height: usize,
}

impl Size {
    /// Creates a size from its width and height.
    #[must_use]
    pub const fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn extent_parallel_to(self, direction: Direction) -> usize {
        match direction {
            Direction::Row => self.width,
            Direction::Column => self.height,
        }
    }

    #[must_use]
    pub const fn extent_perpendicular_to(self, direction: Direction) -> usize {
        match direction {
            Direction::Row => self.height,
            Direction::Column => self.width,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// A rectangular region in the terminal.
pub struct Rect {
    /// The horizontal origin in cells.
    pub x: usize,
    /// The vertical origin in cells.
    pub y: usize,
    /// The rectangle width in cells.
    pub width: usize,
    /// The rectangle height in cells.
    pub height: usize,
}

impl Rect {
    /// Creates a rectangle from its origin and dimensions.
    #[must_use]
    pub const fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Minimum and maximum size for one dimension.
pub struct AxisConstraint {
    /// The minimum size in cells.
    pub min: usize,
    /// The maximum size in cells.
    pub max: usize,
}

impl AxisConstraint {
    /// Creates an unconstrained dimension up to the supplied maximum.
    #[must_use]
    pub const fn at_most(max: usize) -> Self {
        Self { min: 0, max }
    }

    /// Creates a dimension with an exact size.
    #[must_use]
    pub const fn exact(size: usize) -> Self {
        Self {
            min: size,
            max: size,
        }
    }

    /// Creates a dimension with a minimum and maximum size.
    #[must_use]
    pub const fn range(min: usize, max: usize) -> Self {
        Self { min, max }
    }

    /// Intersects this dimension with another dimension.
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        let max = self.max.min(other.max);
        Self {
            min: self.min.max(other.min).min(max),
            max,
        }
    }

    /// Clamps a measured dimension to this range.
    #[must_use]
    pub fn clamp(self, size: usize) -> usize {
        size.max(self.min).min(self.max)
    }

    /// Reduces the dimension by space occupied by an enclosing element.
    #[must_use]
    pub const fn shrink(self, amount: usize) -> Self {
        Self {
            min: self.min.saturating_sub(amount),
            max: self.max.saturating_sub(amount),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Minimum and maximum dimensions supplied when measuring a view.
pub struct Constraints {
    /// Width constraints in cells.
    pub width: AxisConstraint,
    /// Height constraints in cells.
    pub height: AxisConstraint,
}

impl Constraints {
    /// Creates constraints with independent width and height ranges.
    #[must_use]
    pub const fn new(width: AxisConstraint, height: AxisConstraint) -> Self {
        Self { width, height }
    }

    /// Creates constraints limited only by maximum width and height.
    #[must_use]
    pub const fn at_most(max_width: usize, max_height: usize) -> Self {
        Self::new(
            AxisConstraint::at_most(max_width),
            AxisConstraint::at_most(max_height),
        )
    }

    /// Creates constraints requiring an exact width and height.
    #[must_use]
    pub const fn exact(width: usize, height: usize) -> Self {
        Self::new(AxisConstraint::exact(width), AxisConstraint::exact(height))
    }

    /// Intersects these constraints with another set of constraints.
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        Self::new(
            self.width.intersect(other.width),
            self.height.intersect(other.height),
        )
    }

    /// Clamps a measured size to these constraints.
    #[must_use]
    pub fn clamp(self, size: Size) -> Size {
        Size::new(self.width.clamp(size.width), self.height.clamp(size.height))
    }

    /// Reduces both dimensions by space occupied by an enclosing element.
    #[must_use]
    pub const fn shrink(self, width: usize, height: usize) -> Self {
        Self::new(self.width.shrink(width), self.height.shrink(height))
    }

    /// Sets the minimum width while retaining the current maximum.
    #[must_use]
    pub const fn min_width(mut self, width: usize) -> Self {
        self.width.min = if width < self.width.max {
            width
        } else {
            self.width.max
        };
        self
    }

    /// Sets the maximum width while retaining the current minimum.
    #[must_use]
    pub const fn max_width(mut self, width: usize) -> Self {
        self.width.max = width;
        if self.width.min > width {
            self.width.min = width;
        }
        self
    }

    /// Sets the minimum height while retaining the current maximum.
    #[must_use]
    pub const fn min_height(mut self, height: usize) -> Self {
        self.height.min = if height < self.height.max {
            height
        } else {
            self.height.max
        };
        self
    }

    /// Sets the maximum height while retaining the current minimum.
    #[must_use]
    pub const fn max_height(mut self, height: usize) -> Self {
        self.height.max = height;
        if self.height.min > height {
            self.height.min = height;
        }
        self
    }

    /// Sets the exact width.
    #[must_use]
    pub const fn width(self, width: usize) -> Self {
        Self {
            width: AxisConstraint::exact(width),
            ..self
        }
    }

    /// Sets the exact height.
    #[must_use]
    pub const fn height(self, height: usize) -> Self {
        Self {
            height: AxisConstraint::exact(height),
            ..self
        }
    }
}

/// A terminal view that can be measured and rendered.
pub trait View {
    /// Computes the desired size under the supplied constraints.
    fn measure(&mut self, constraints: Constraints) -> Size;

    /// Returns the resolved style values consumed by parent layouts.
    fn style(&self) -> &ViewStyle;

    /// Returns mutable style values for fluent view configuration.
    fn style_mut(&mut self) -> &mut ViewStyle;

    /// Assigns this view its final rectangle and arranges its children.
    fn arrange(&mut self, bounds: Rect);

    /// Paints the already-arranged view.
    fn render(&self, buffer: &mut Buffer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constraints_intersect_and_clamp() {
        let constraints = Constraints::new(AxisConstraint::range(4, 12), AxisConstraint::exact(3))
            .intersect(Constraints::at_most(8, 10));

        assert_eq!(constraints.width, AxisConstraint::range(4, 8));
        assert_eq!(constraints.height, AxisConstraint::exact(3));
        assert_eq!(constraints.clamp(Size::new(2, 9)), Size::new(4, 3));
    }

    #[test]
    fn exact_constraint_becomes_feasible_against_parent_limit() {
        let constraints = Constraints::exact(10, 4).intersect(Constraints::at_most(6, 8));

        assert_eq!(constraints, Constraints::exact(6, 4));
    }
}
