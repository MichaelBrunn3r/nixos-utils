use crate::buffer::{Buffer, Style};
use crate::view::{Constraints, Rect, Size, View};

const DEFAULT_WIDTH: usize = 20;
const PARTIAL_BLOCKS: [char; 8] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

/// A horizontal progress bar that fills the width offered by its parent.
pub struct ProgressBar {
    progress: f64,
    filled_style: Style,
    empty_style: Style,
    partial_blocks: Vec<char>,
    label: String,
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
            partial_blocks: PARTIAL_BLOCKS.to_vec(),
            label: String::new(),
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

    /// Sets the characters used for fractional cells.
    #[must_use]
    pub fn partial_blocks(mut self, partial_blocks: Vec<char>) -> Self {
        self.partial_blocks = partial_blocks;
        self
    }

    /// Sets the centered label rendered over the bar.
    ///
    /// Labels wider than the bar are clipped.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
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
        let width = if constraints.width.max == usize::MAX {
            DEFAULT_WIDTH
        } else {
            constraints.width.max
        };
        constraints.clamp(Size::new(width, 1))
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

        let filled = (bounds.width as f64) * self.progress;
        let filled_width = filled.floor() as usize;
        let partial = filled - filled_width as f64;
        let partial_block = if filled_width < bounds.width
            && partial > 0.0
            && !self.partial_blocks.is_empty()
        {
            let index = (partial * self.partial_blocks.len() as f64).ceil() as usize;
            Some(self.partial_blocks[index.saturating_sub(1).min(self.partial_blocks.len() - 1)])
        } else {
            None
        };

        for (index, x) in (bounds.x..bounds.x.saturating_add(bounds.width)).enumerate() {
            if let Some(cell) = buffer.cell_mut(x, bounds.y) {
                cell.ch = ' ';
                cell.style = if index < filled_width {
                    self.filled_style
                } else {
                    self.empty_style
                };

                if index == filled_width
                    && let Some(partial_block) = partial_block
                {
                    cell.ch = partial_block;
                    cell.style = self
                        .filled_style
                        .background()
                        .map_or(self.filled_style, |color| self.empty_style.with_fg(color));
                }
            }
        }

        let label_width = self.label.chars().count().min(bounds.width);
        let label_start = bounds.width.saturating_sub(label_width) / 2;
        for (index, character) in self.label.chars().take(label_width).enumerate() {
            if let Some(cell) = buffer.cell_mut(
                bounds.x.saturating_add(label_start).saturating_add(index),
                bounds.y,
            ) {
                cell.ch = character;
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

#[cfg(test)]
#[allow(clippy::cast_precision_loss)]
mod tests {
    use super::*;
    use crate::test_utils::render_sized;

    #[test]
    fn fills_available_width_and_one_row() {
        let mut progress = ProgressBar::new();

        assert_eq!(
            progress.measure(Constraints::at_most(12, 4)),
            Size::new(12, 1)
        );
    }

    #[test]
    fn uses_intrinsic_width_when_unbounded() {
        let mut progress = ProgressBar::new();

        assert_eq!(
            progress.measure(Constraints::at_most(usize::MAX, usize::MAX)),
            Size::new(DEFAULT_WIDTH, 1)
        );
    }

    #[test]
    fn snapshots_progress_variants() {
        let mut snapshots = vec![(
            "empty".to_owned(),
            render_sized(ProgressBar::new().progress(-1.0), (10, 1)),
        )];
        for partial in 1..PARTIAL_BLOCKS.len() {
            let progress = (3.0 + partial as f64 / PARTIAL_BLOCKS.len() as f64) / 10.0;
            snapshots.push((
                format!("fraction {partial}/8"),
                render_sized(ProgressBar::new().progress(progress), (10, 1)),
            ));
        }
        snapshots.push((
            "complete".to_owned(),
            render_sized(ProgressBar::new().progress(2.0), (10, 1)),
        ));

        let snapshots = snapshots
            .into_iter()
            .map(|(name, output)| format!("{name}\n{output}"))
            .collect::<Vec<_>>()
            .join("\n\n");

        insta::assert_snapshot!(snapshots);
    }

    #[test]
    fn snapshots_progress_widths() {
        let snapshots = [5, 10, 20, 50]
            .into_iter()
            .map(|width| {
                (
                    format!("25% at width {width}"),
                    render_sized(ProgressBar::new().progress(0.25), (width, 1)),
                )
            })
            .map(|(name, output)| format!("{name}\n{output}"))
            .collect::<Vec<_>>()
            .join("\n\n");

        insta::assert_snapshot!(snapshots);
    }
}
