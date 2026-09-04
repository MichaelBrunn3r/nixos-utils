use crate::buffer::{Buffer, Style};
use crate::view::{Constraints, Rect, Size, View};

pub struct Span {
    data: String,
    style: Style,
    bounds: Option<Rect>,
}

impl Span {
    #[must_use]
    pub fn new(data: impl Into<String>) -> Self {
        Self::styled(data, Style::default())
    }

    #[must_use]
    pub fn styled(data: impl Into<String>, style: Style) -> Self {
        Self {
            data: data.into(),
            style,
            bounds: None,
        }
    }
}

impl View for Span {
    fn measure(&mut self, constraints: Constraints) -> Size {
        if constraints.width.max == 0 || constraints.height.max == 0 {
            return Size::new(0, 0);
        }
        let width = constraints.width.max.min(self.data.chars().count());
        let height = usize::from(width > 0);
        constraints.clamp(Size { width, height })
    }

    fn arrange(&mut self, bounds: Rect) {
        self.bounds = Some(bounds);
    }

    fn render(&self, buffer: &mut Buffer) {
        let Some(bounds) = self.bounds else {
            return;
        };
        let x_end = bounds.x.saturating_add(bounds.width);
        for (x, ch) in (bounds.x..x_end).zip(self.data.chars()) {
            if let Some(cell) = buffer.cell_mut(x, bounds.y) {
                cell.ch = ch;
                cell.style = self.style;
            }
        }
    }
}
