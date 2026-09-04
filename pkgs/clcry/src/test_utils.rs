//! Shared scaffolding for layout-view unit tests (compiled for tests only).

use crate::Style;
use crate::buffer::Buffer;
use crate::span::Span;
use crate::view::{Constraints, Rect, View};
use crate::{BorderStyle, Frame};

pub fn create_item_spans(n: usize) -> Vec<Box<dyn View>> {
    (0..n)
        .map(|i| Box::new(Span::new(format!("Item{i}"))) as Box<dyn View>)
        .collect()
}

/// Renders a view at a fixed size inside a one-cell border.
pub fn render_sized(root: impl View + 'static, (width, height): (usize, usize)) -> String {
    let mut buffer = Buffer::new(width.saturating_add(2), height.saturating_add(2));
    let mut root = Frame::new(root).border(BorderStyle {
        top_left: '#',
        top_right: '#',
        bottom_left: '#',
        bottom_right: '#',
        horizontal: '#',
        vertical: '#',
        style: Style::default(),
    });
    let bounds = Rect::new(0, 0, buffer.width(), buffer.height());
    let _ = root.measure(Constraints::at_most(bounds.width, bounds.height));
    root.arrange(bounds);
    root.render(&mut buffer);
    let plain = buffer.to_plain();
    let mut lines: Vec<&str> = plain.split('\n').collect();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

/// Renders a view at its measured size.
pub fn render(root: impl View + 'static) -> String {
    let mut root = Frame::new(root);
    let size = root.measure(Constraints::at_most(usize::MAX, usize::MAX));
    let mut buffer = Buffer::new(size.width, size.height);
    root.arrange(Rect::new(0, 0, size.width, size.height));
    root.render(&mut buffer);
    buffer.to_plain()
}
