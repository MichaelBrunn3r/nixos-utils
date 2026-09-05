use crate::buffer::{Buffer, Style};
use crate::view::{Constraints, Rect, Size};

/// Space applied to each side of a rectangular region.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Insets {
    /// Space above the region.
    pub top: usize,
    /// Space to the right of the region.
    pub right: usize,
    /// Space below the region.
    pub bottom: usize,
    /// Space to the left of the region.
    pub left: usize,
}

impl Insets {
    /// Creates independent insets for each side.
    #[must_use]
    pub const fn new(top: usize, right: usize, bottom: usize, left: usize) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Creates equal insets on every side.
    #[must_use]
    pub const fn all(value: usize) -> Self {
        Self::new(value, value, value, value)
    }

    /// Creates equal horizontal and vertical insets.
    #[must_use]
    pub const fn symmetric(horizontal: usize, vertical: usize) -> Self {
        Self::new(vertical, horizontal, vertical, horizontal)
    }

    const fn horizontal(self) -> usize {
        self.left.saturating_add(self.right)
    }

    const fn vertical(self) -> usize {
        self.top.saturating_add(self.bottom)
    }

    fn inset_rect(self, bounds: Rect) -> Rect {
        let left = self.left.min(bounds.width);
        let right = self.right.min(bounds.width.saturating_sub(left));
        let top = self.top.min(bounds.height);
        let bottom = self.bottom.min(bounds.height.saturating_sub(top));

        Rect::new(
            bounds.x.saturating_add(left),
            bounds.y.saturating_add(top),
            bounds.width.saturating_sub(left).saturating_sub(right),
            bounds.height.saturating_sub(top).saturating_sub(bottom),
        )
    }
}

/// Characters and style used to draw a view border.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BorderStyle {
    /// Character for the top-left corner.
    pub top_left: char,
    /// Character for the top-right corner.
    pub top_right: char,
    /// Character for the bottom-left corner.
    pub bottom_left: char,
    /// Character for the bottom-right corner.
    pub bottom_right: char,
    /// Character for horizontal edges.
    pub horizontal: char,
    /// Character for vertical edges.
    pub vertical: char,
    /// Style applied to border cells.
    pub style: Style,
}

impl Default for BorderStyle {
    fn default() -> Self {
        Self {
            top_left: '+',
            top_right: '+',
            bottom_left: '+',
            bottom_right: '+',
            horizontal: '-',
            vertical: '|',
            style: Style::default(),
        }
    }
}

/// Resolved outer, border, and content rectangles for a view.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct BoxGeometry {
    pub(crate) outer: Rect,
    pub(crate) border: Rect,
    pub(crate) content: Rect,
}

/// Resolved style values consumed by layout and rendering.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ViewStyle {
    constraints: Constraints,
    flex_grow: usize,
    margin: Insets,
    padding: Insets,
    border: Option<BorderStyle>,
}

impl Default for ViewStyle {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewStyle {
    /// Creates a style with no layout or paint overrides.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            constraints: Constraints::at_most(usize::MAX, usize::MAX),
            flex_grow: 0,
            margin: Insets::all(0),
            padding: Insets::all(0),
            border: None,
        }
    }

    /// Returns the explicit size constraints for the view.
    #[must_use]
    pub const fn constraints(self) -> Constraints {
        self.constraints
    }

    /// Resolves this style's constraints against those supplied by its parent.
    #[must_use]
    pub fn resolve(self, parent: Constraints) -> Constraints {
        parent.intersect(self.constraints)
    }

    pub(crate) fn content_constraints(self, constraints: Constraints) -> Constraints {
        self.resolve(constraints).shrink(
            self.total_insets().horizontal(),
            self.total_insets().vertical(),
        )
    }

    pub(crate) fn outer_size(self, content: Size) -> Size {
        Size::new(
            content
                .width
                .saturating_add(self.total_insets().horizontal()),
            content
                .height
                .saturating_add(self.total_insets().vertical()),
        )
    }

    pub(crate) fn geometry(self, outer: Rect) -> BoxGeometry {
        let border = self.margin.inset_rect(outer);
        let content = self.border_insets().inset_rect(border);
        BoxGeometry {
            outer,
            border,
            content: self.padding.inset_rect(content),
        }
    }

    pub(crate) fn render_decorations(self, buffer: &mut Buffer, geometry: BoxGeometry) {
        if let Some(style) = self.border {
            render_border(buffer, geometry.border, style);
        }
    }

    fn border_insets(self) -> Insets {
        if self.border.is_some() {
            Insets::all(1)
        } else {
            Insets::default()
        }
    }

    fn total_insets(self) -> Insets {
        let border = self.border_insets();
        Insets::new(
            self.margin
                .top
                .saturating_add(border.top)
                .saturating_add(self.padding.top),
            self.margin
                .right
                .saturating_add(border.right)
                .saturating_add(self.padding.right),
            self.margin
                .bottom
                .saturating_add(border.bottom)
                .saturating_add(self.padding.bottom),
            self.margin
                .left
                .saturating_add(border.left)
                .saturating_add(self.padding.left),
        )
    }

    /// Returns the relative share of spare linear-layout space.
    #[must_use]
    pub const fn flex_grow(self) -> usize {
        self.flex_grow
    }
}

/// Fluent setters for the style owned by a view.
pub trait ViewStyleExt: crate::view::View + Sized {
    /// Sets the relative share of spare linear-layout space.
    #[must_use]
    fn flex_grow(mut self, flex_grow: usize) -> Self {
        self.style_mut().flex_grow = flex_grow;
        self
    }

    /// Sets the outer margin.
    #[must_use]
    fn margin(mut self, margin: Insets) -> Self {
        self.style_mut().margin = margin;
        self
    }

    /// Sets the inner padding.
    #[must_use]
    fn padding(mut self, padding: Insets) -> Self {
        self.style_mut().padding = padding;
        self
    }

    /// Sets the border style.
    #[must_use]
    fn border(mut self, border: BorderStyle) -> Self {
        self.style_mut().border = Some(border);
        self
    }

    /// Sets the exact width.
    #[must_use]
    fn width(mut self, width: usize) -> Self {
        self.style_mut().constraints = self.style().constraints().width(width);
        self
    }

    /// Sets the exact height.
    #[must_use]
    fn height(mut self, height: usize) -> Self {
        self.style_mut().constraints = self.style().constraints().height(height);
        self
    }

    /// Sets the minimum width.
    #[must_use]
    fn min_width(mut self, width: usize) -> Self {
        self.style_mut().constraints = self.style().constraints().min_width(width);
        self
    }

    /// Sets the minimum height.
    #[must_use]
    fn min_height(mut self, height: usize) -> Self {
        self.style_mut().constraints = self.style().constraints().min_height(height);
        self
    }

    /// Sets the maximum width.
    #[must_use]
    fn max_width(mut self, width: usize) -> Self {
        self.style_mut().constraints = self.style().constraints().max_width(width);
        self
    }

    /// Sets the maximum height.
    #[must_use]
    fn max_height(mut self, height: usize) -> Self {
        self.style_mut().constraints = self.style().constraints().max_height(height);
        self
    }
}

impl<V: crate::view::View + Sized> ViewStyleExt for V {}

fn render_border(buffer: &mut Buffer, bounds: Rect, style: BorderStyle) {
    if bounds.width == 0 || bounds.height == 0 {
        return;
    }

    let right = bounds.x.saturating_add(bounds.width - 1);
    let bottom = bounds.y.saturating_add(bounds.height - 1);

    for x in bounds.x..=right {
        if let Some(cell) = buffer.cell_mut(x, bounds.y) {
            cell.ch = if x == bounds.x {
                style.top_left
            } else if x == right {
                style.top_right
            } else {
                style.horizontal
            };
            cell.style = style.style;
        }
        if bottom != bounds.y
            && let Some(cell) = buffer.cell_mut(x, bottom)
        {
            cell.ch = if x == bounds.x {
                style.bottom_left
            } else if x == right {
                style.bottom_right
            } else {
                style.horizontal
            };
            cell.style = style.style;
        }
    }

    for y in bounds.y.saturating_add(1)..bottom {
        if let Some(cell) = buffer.cell_mut(bounds.x, y) {
            cell.ch = style.vertical;
            cell.style = style.style;
        }
        if right != bounds.x
            && let Some(cell) = buffer.cell_mut(right, y)
        {
            cell.ch = style.vertical;
            cell.style = style.style;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{Size, View};
    use crate::{Span, ViewStyleExt};

    #[test]
    fn applies_exact_minimum_and_maximum_constraints() {
        let parent = Constraints::at_most(20, 20);

        assert_eq!(
            Span::new("hello").width(10).measure(parent),
            Size::new(10, 1)
        );
        assert_eq!(
            Span::new("hello").min_width(8).measure(parent),
            Size::new(8, 1)
        );
        assert_eq!(
            Span::new("hello").max_width(3).measure(parent),
            Size::new(3, 1)
        );
    }

    #[test]
    fn local_constraints_are_clamped_to_parent_constraints() {
        let size = Span::new("hello")
            .width(20)
            .measure(Constraints::at_most(8, 1));

        assert_eq!(size, Size::new(8, 1));
    }

    #[test]
    fn box_model_adds_margin_border_and_padding_to_outer_size() {
        let view = Span::new("hello")
            .margin(Insets::all(1))
            .border(BorderStyle::default())
            .padding(Insets::all(2));

        assert_eq!(view.style().outer_size(Size::new(5, 1)), Size::new(13, 9));
    }

    #[test]
    fn exact_width_includes_box_model() {
        let mut view = Span::new("hello")
            .margin(Insets::all(1))
            .border(BorderStyle::default())
            .padding(Insets::all(2))
            .width(30);

        assert_eq!(view.measure(Constraints::at_most(40, 10)), Size::new(30, 9));
    }
}
