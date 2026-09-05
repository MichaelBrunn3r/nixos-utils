use crate::buffer::{Buffer, Style};
use crate::style::ViewStyle;
use crate::view::{Constraints, Rect, Size, View};

pub struct Span {
    data: String,
    style: Style,
    view_style: ViewStyle,
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
            view_style: ViewStyle::new(),
            bounds: None,
        }
    }
}

impl View for Span {
    fn measure(&mut self, constraints: Constraints) -> Size {
        let outer_constraints = self.view_style.resolve(constraints);
        let content_constraints = self.view_style.content_constraints(constraints);
        if content_constraints.width.max == 0 || content_constraints.height.max == 0 {
            return Size::new(0, 0);
        }
        let width = content_constraints.width.max.min(self.data.chars().count());
        let height = usize::from(width > 0);
        outer_constraints.clamp(self.view_style.outer_size(Size { width, height }))
    }

    fn style(&self) -> &ViewStyle {
        &self.view_style
    }

    fn style_mut(&mut self) -> &mut ViewStyle {
        &mut self.view_style
    }

    fn arrange(&mut self, bounds: Rect) {
        self.bounds = Some(bounds);
    }

    fn render(&self, buffer: &mut Buffer) {
        let Some(bounds) = self.bounds else {
            return;
        };
        let geometry = self.view_style.geometry(bounds);
        self.view_style.render_decorations(buffer, geometry);
        let x_end = geometry.content.x.saturating_add(geometry.content.width);
        for (x, ch) in (geometry.content.x..x_end).zip(self.data.chars()) {
            if let Some(cell) = buffer.cell_mut(x, geometry.content.y) {
                cell.ch = ch;
                cell.style = self.style;
            }
        }
    }
}
