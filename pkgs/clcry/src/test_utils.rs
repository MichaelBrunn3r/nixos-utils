//! Shared scaffolding for layout-view unit tests (compiled for tests only).

use crate::buffer::Buffer;
use crate::view::{Constraints, Rect, View};

pub fn create_item_spans(n: usize) -> Vec<Box<dyn View>> {
    (0..n)
        .map(|i| Box::new(crate::span!(format!("Item{i}"))) as Box<dyn View>)
        .collect()
}

/// Renders a view at its measured size.
pub fn render(root: impl View + 'static) -> String {
    let mut root = root;
    let size = root.measure(Constraints::at_most(usize::MAX, usize::MAX));
    let mut buffer = Buffer::new(size.width, size.height);
    root.arrange(Rect::new(0, 0, size.width, size.height));
    root.render(&mut buffer);
    buffer.to_plain()
}
