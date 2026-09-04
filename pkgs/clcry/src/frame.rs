use crate::buffer::{Buffer, Style};
use crate::view::{Constraints, Rect, Size, View};

/// A view frame with optional margin, border, and padding.
pub struct Frame {
    child: Box<dyn View>,
    margin: Insets,
    padding: Insets,
    border: Option<BorderStyle>,
    bounds: Option<Rect>,
}

impl Frame {
    /// Creates a frame around a view.
    #[must_use]
    pub fn new(child: impl View + 'static) -> Self {
        Self {
            child: Box::new(child),
            margin: Insets::default(),
            padding: Insets::default(),
            border: None,
            bounds: None,
        }
    }

    /// Sets the outer margin.
    #[must_use]
    pub const fn margin(mut self, margin: Insets) -> Self {
        self.margin = margin;
        self
    }

    /// Sets the inner padding.
    #[must_use]
    pub const fn padding(mut self, padding: Insets) -> Self {
        self.padding = padding;
        self
    }

    /// Adds a one-cell border with the supplied style.
    #[must_use]
    pub const fn border(mut self, style: BorderStyle) -> Self {
        self.border = Some(style);
        self
    }

    fn border_insets(&self) -> Insets {
        if self.border.is_some() {
            Insets::all(1)
        } else {
            Insets::default()
        }
    }

    fn content_insets(&self) -> Insets {
        let border = self.border_insets();
        Insets::new(
            border.top.saturating_add(self.padding.top),
            border.right.saturating_add(self.padding.right),
            border.bottom.saturating_add(self.padding.bottom),
            border.left.saturating_add(self.padding.left),
        )
    }

    fn total_insets(&self) -> Insets {
        let content = self.content_insets();
        Insets::new(
            self.margin.top.saturating_add(content.top),
            self.margin.right.saturating_add(content.right),
            self.margin.bottom.saturating_add(content.bottom),
            self.margin.left.saturating_add(content.left),
        )
    }
}

impl View for Frame {
    fn measure(&mut self, constraints: Constraints) -> Size {
        let child_size = self
            .child
            .measure(self.total_insets().shrink_constraints(constraints));
        let insets = self.total_insets();
        constraints.clamp(Size::new(
            child_size.width.saturating_add(insets.horizontal()),
            child_size.height.saturating_add(insets.vertical()),
        ))
    }

    fn arrange(&mut self, bounds: Rect) {
        self.bounds = Some(bounds);
        let border_bounds = self.margin.inset_rect(bounds);
        let content_bounds = self.border_insets().inset_rect(border_bounds);
        self.child.arrange(self.padding.inset_rect(content_bounds));
    }

    fn render(&self, buffer: &mut Buffer) {
        let Some(bounds) = self.bounds else {
            return;
        };

        if let Some(style) = self.border {
            render_border(buffer, self.margin.inset_rect(bounds), style);
        }
        self.child.render(buffer);
    }
}

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

    const fn shrink_constraints(self, constraints: Constraints) -> Constraints {
        constraints.shrink(self.horizontal(), self.vertical())
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

/// Characters and style used to draw a frame border.
#[derive(Copy, Clone)]
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
    use crate::span::Span;
    use crate::test_utils::render;

    #[test]
    fn snapshots_frame_variants() {
        fn bordered(frame: impl View + 'static) -> Frame {
            Frame::new(frame).border(BorderStyle::default())
        }

        let layout = crate::vstack![
            Span::new("Border"),
            Frame::new(Span::new("hello")).border(BorderStyle::default()),
            Span::new("Border+Margin"),
            bordered(
                Frame::new(Span::new("hello"))
                    .margin(Insets::all(2))
                    .border(BorderStyle::default()),
            ),
            Span::new("Border+Padding"),
            bordered(
                Frame::new(Span::new("hello"))
                    .border(BorderStyle::default())
                    .padding(Insets::all(2)),
            ),
            Span::new("Border+Margin+Padding"),
            bordered(
                Frame::new(Span::new("hello"))
                    .margin(Insets::all(1))
                    .border(BorderStyle::default())
                    .padding(Insets::all(1)),
            ),
            Span::new("Max width & height = margin + border + padding + content"),
            bordered(
                crate::Sized::new(
                    Frame::new(crate::vstack![
                        Span::new("0123456789"),
                        Span::new("abcdefghij"),
                        Span::new("klmnopqrst"),
                        Span::new("uvwxyzABCD"),
                        Span::new("EFGHIJKLMN"),
                    ])
                    .margin(Insets::all(1))
                    .border(BorderStyle::default())
                    .padding(Insets::all(1)),
                )
                .max_width(10)
                .max_height(10),
            ),
            Span::new("Oversized insets leave no child space"),
            bordered(
                crate::Sized::new(Frame::new(Span::new("hello")).padding(Insets::all(10)))
                    .max_width(5)
                    .max_height(5),
            ),
        ];
        insta::assert_snapshot!(render(layout));
    }

    #[test]
    fn supports_exact_and_minimum_dimensions() {
        let mut exact =
            crate::Sized::new(Frame::new(Span::new("hello")).border(BorderStyle::default()))
                .width(10)
                .height(4);
        assert_eq!(
            exact.measure(Constraints::at_most(20, 20)),
            Size::new(10, 4)
        );

        let mut minimum = crate::Sized::new(Frame::new(Span::new("hello")))
            .min_width(12)
            .min_height(3);
        assert_eq!(
            minimum.measure(Constraints::at_most(20, 20)),
            Size::new(12, 3)
        );
    }
}
