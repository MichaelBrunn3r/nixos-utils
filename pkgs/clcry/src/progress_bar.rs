use crate::buffer::{Buffer, Style};
use crate::style::ViewStyle;
use crate::view::{Constraints, Rect, Size, View};

const DEFAULT_WIDTH: usize = 20;
const GLYPH: char = '■';

/// A horizontal progress bar that fills the width offered by its parent.
pub struct ProgressBar {
    progress: f64,
    filled_style: Style,
    empty_style: Style,
    view_style: ViewStyle,
    bounds: Option<Rect>,
}

impl ProgressBar {
    /// Creates an empty progress bar.
    #[must_use]
    pub fn new() -> Self {
        Self {
            progress: 0.0,
            filled_style: Style::default(),
            empty_style: Style::default(),
            view_style: ViewStyle::new(),
            bounds: None,
        }
    }

    /// Sets the progress as a value between zero and one.
    #[must_use]
    pub const fn progress(mut self, progress: f64) -> Self {
        self.progress = normalize_progress(progress);
        self
    }

    /// Sets the style used for completed cells.
    #[must_use]
    pub const fn filled_style(mut self, style: Style) -> Self {
        self.filled_style = style;
        self
    }

    /// Sets the style used for incomplete cells.
    #[must_use]
    pub const fn empty_style(mut self, style: Style) -> Self {
        self.empty_style = style;
        self
    }
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self::new()
    }
}

impl View for ProgressBar {
    fn measure(&mut self, constraints: Constraints) -> Size {
        let outer_constraints = self.view_style.resolve(constraints);
        let content_constraints = self.view_style.content_constraints(constraints);
        let width = if content_constraints.width.max == usize::MAX {
            DEFAULT_WIDTH
        } else {
            content_constraints.width.max
        };
        outer_constraints.clamp(self.view_style.outer_size(Size::new(width, 1)))
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

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    fn render(&self, buffer: &mut Buffer) {
        let Some(bounds) = self.bounds else {
            return;
        };

        let geometry = self.view_style.geometry(bounds);
        self.view_style.render_decorations(buffer, geometry);
        let filled_width = ((geometry.content.width as f64) * self.progress).floor() as usize;

        for (index, x) in (geometry.content.x
            ..geometry.content.x.saturating_add(geometry.content.width))
            .enumerate()
        {
            if let Some(cell) = buffer.cell_mut(x, geometry.content.y) {
                cell.ch = GLYPH;
                cell.style = if index < filled_width {
                    self.filled_style
                } else {
                    self.empty_style
                };
            }
        }
    }
}

const fn normalize_progress(progress: f64) -> f64 {
    if progress.is_finite() {
        progress.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
