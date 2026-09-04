use crate::buffer::Buffer;
use crate::view::{Constraints, Rect, Size, View};

/// Sizing behavior for a grid column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridTrack {
    /// Size the column to its widest cell.
    Content,
    /// Give the column its minimum content width plus a share of spare space.
    Flexible(usize),
}

/// A two-dimensional layout with shared column widths and row heights.
pub struct Grid {
    rows: Vec<Vec<Box<dyn View>>>,
    tracks: Vec<GridTrack>,
    column_gap: usize,
    row_gap: usize,
    intrinsic_widths: Vec<usize>,
    row_heights: Vec<usize>,
    column_widths: Vec<usize>,
}

impl Grid {
    /// Creates a grid from rows of views.
    #[must_use]
    pub fn new(rows: Vec<Vec<Box<dyn View>>>) -> Self {
        Self {
            rows,
            tracks: Vec::new(),
            column_gap: 0,
            row_gap: 0,
            intrinsic_widths: Vec::new(),
            row_heights: Vec::new(),
            column_widths: Vec::new(),
        }
    }

    /// Sets the sizing behavior for each column.
    #[must_use]
    pub fn columns(mut self, tracks: impl IntoIterator<Item = GridTrack>) -> Self {
        self.tracks = tracks.into_iter().collect();
        self
    }

    /// Sets the space between columns.
    #[must_use]
    pub const fn column_gap(mut self, gap: usize) -> Self {
        self.column_gap = gap;
        self
    }

    /// Sets the space between rows.
    #[must_use]
    pub const fn row_gap(mut self, gap: usize) -> Self {
        self.row_gap = gap;
        self
    }

    fn column_count(&self) -> usize {
        self.rows.iter().map(Vec::len).max().unwrap_or(0)
    }

    fn track(&self, column: usize) -> GridTrack {
        self.tracks
            .get(column)
            .copied()
            .unwrap_or(GridTrack::Content)
    }

    fn resolve_widths(&self, available: usize, intrinsic: &[usize]) -> Vec<usize> {
        let mut widths = intrinsic.to_vec();
        let gaps = self
            .column_gap
            .saturating_mul(widths.len().saturating_sub(1));
        let minimum = widths.iter().sum::<usize>().saturating_add(gaps);
        if available > minimum {
            let remaining = available - minimum;
            let weight = (0..widths.len())
                .map(|column| match self.track(column) {
                    GridTrack::Content => 0,
                    GridTrack::Flexible(weight) => weight,
                })
                .sum::<usize>();
            if weight > 0 {
                let mut distributed = 0usize;
                for (column, width) in widths.iter_mut().enumerate() {
                    let allocation = match self.track(column) {
                        GridTrack::Content => 0,
                        GridTrack::Flexible(track_weight) => remaining
                            .saturating_mul(track_weight)
                            .checked_div(weight)
                            .unwrap_or_default(),
                    };
                    *width = width.saturating_add(allocation);
                    distributed = distributed.saturating_add(allocation);
                }
                let remainder = remaining.saturating_sub(distributed);
                if remainder > 0
                    && let Some(width) =
                        widths.iter_mut().enumerate().find_map(|(column, width)| {
                            matches!(self.track(column), GridTrack::Flexible(weight) if weight > 0)
                                .then_some(width)
                        })
                {
                    *width = width.saturating_add(remainder);
                }
            }
        } else if available < minimum {
            let mut excess = minimum - available;
            for width in widths.iter_mut().rev() {
                let reduction = (*width).min(excess);
                *width -= reduction;
                excess -= reduction;
                if excess == 0 {
                    break;
                }
            }
        }
        widths
    }

    fn total_size(&self, widths: &[usize], heights: &[usize]) -> Size {
        Size::new(
            widths.iter().sum::<usize>().saturating_add(
                self.column_gap
                    .saturating_mul(widths.len().saturating_sub(1)),
            ),
            heights
                .iter()
                .sum::<usize>()
                .saturating_add(self.row_gap.saturating_mul(heights.len().saturating_sub(1))),
        )
    }
}

impl View for Grid {
    fn measure(&mut self, constraints: Constraints) -> Size {
        let column_count = self.column_count();
        let mut intrinsic_widths = vec![0; column_count];
        let mut row_heights = Vec::with_capacity(self.rows.len());

        for row in &mut self.rows {
            let mut row_height = 0;
            for (column, cell) in row.iter_mut().enumerate() {
                let size = cell.measure(Constraints::at_most(usize::MAX, constraints.height.max));
                intrinsic_widths[column] = intrinsic_widths[column].max(size.width);
                row_height = row_height.max(size.height);
            }
            row_heights.push(row_height);
        }

        self.intrinsic_widths = intrinsic_widths;
        self.row_heights = row_heights;
        self.column_widths = self.resolve_widths(constraints.width.max, &self.intrinsic_widths);

        for (row_index, row) in self.rows.iter_mut().enumerate() {
            for (column, cell) in row.iter_mut().enumerate() {
                cell.measure(Constraints::exact(
                    self.column_widths[column],
                    self.row_heights[row_index],
                ));
            }
        }

        constraints.clamp(self.total_size(&self.column_widths, &self.row_heights))
    }

    fn arrange(&mut self, bounds: Rect) {
        self.column_widths = self.resolve_widths(bounds.width, &self.intrinsic_widths);
        let mut y = bounds.y;
        for (row_index, row) in self.rows.iter_mut().enumerate() {
            let mut x = bounds.x;
            for (column, cell) in row.iter_mut().enumerate() {
                cell.arrange(Rect::new(
                    x,
                    y,
                    self.column_widths[column],
                    self.row_heights[row_index],
                ));
                x = x
                    .saturating_add(self.column_widths[column])
                    .saturating_add(self.column_gap);
            }
            y = y
                .saturating_add(self.row_heights[row_index])
                .saturating_add(self.row_gap);
        }
    }

    fn render(&self, buffer: &mut Buffer) {
        for row in &self.rows {
            for cell in row {
                cell.render(buffer);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::render;
    use crate::{BorderStyle, Frame, Span};

    fn text_grid(rows: &[&[&str]]) -> Grid {
        Grid::new(
            rows.iter()
                .map(|row| {
                    row.iter()
                        .map(|text| Box::new(Span::new(*text)) as Box<dyn View>)
                        .collect()
                })
                .collect(),
        )
    }

    #[test]
    fn snapshots_grid_cases() {
        let cases: Vec<Box<dyn View>> = vec![
            Box::new(crate::vstack![
                Span::new("1x1"),
                Frame::new(text_grid(&[&["A"][..]])).border(BorderStyle::default()),
            ]),
            Box::new(crate::vstack![
                Span::new("1x3"),
                Frame::new(text_grid(&[&["A", "B", "C"][..]])).border(BorderStyle::default()),
            ]),
            Box::new(crate::vstack![
                Span::new("3x1"),
                Frame::new(text_grid(&[&["A"][..], &["B"][..], &["C"][..]]))
                    .border(BorderStyle::default()),
            ]),
            Box::new(crate::vstack![
                Span::new("2x2"),
                Frame::new(text_grid(&[&["A", "B"][..], &["C", "D"][..]]))
                    .border(BorderStyle::default()),
            ]),
            Box::new(crate::vstack![
                Span::new("2x3 flexible middle column"),
                crate::Sized::new(
                    Frame::new(
                        text_grid(&[&["A", "B", "X"][..], &["BB", "CC", "YYY"][..]])
                            .columns([
                                GridTrack::Content,
                                GridTrack::Flexible(1),
                                GridTrack::Content,
                            ])
                            .column_gap(1),
                    )
                    .border(BorderStyle::default()),
                )
                .width(22),
            ]),
        ];
        insta::assert_snapshot!(render(crate::vstack![..cases].gap(1)));
    }

    #[test]
    fn content_columns_shrink_before_flexible_columns_grow() {
        let mut grid = Grid::new(vec![vec![
            Box::new(Span::new("label")) as Box<dyn View>,
            Box::new(Span::new("middle")),
            Box::new(Span::new("value")),
        ]])
        .columns([
            GridTrack::Content,
            GridTrack::Flexible(1),
            GridTrack::Content,
        ]);

        assert_eq!(grid.measure(Constraints::at_most(30, 1)), Size::new(30, 1));
        assert_eq!(grid.column_widths, vec![5, 20, 5]);
    }
}
