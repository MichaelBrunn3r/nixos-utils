use crate::buffer::Buffer;
use crate::linear_layout::ContentAlignment;
use crate::style::ViewStyle;
use crate::view::{Constraints, Rect, Size, View};

/// Sizing behavior for a grid column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridTrack {
    /// Size the column to its widest cell.
    Content,
    /// Give the column its minimum content width plus a share of spare space.
    Flexible(usize),
}

/// Sizing and alignment behavior for a grid column.
#[derive(Debug, Clone, Copy)]
pub struct GridColumn {
    track: GridTrack,
    alignment: ContentAlignment,
}

impl GridColumn {
    /// Creates a content-sized, left-aligned column.
    #[must_use]
    pub const fn content() -> Self {
        Self {
            track: GridTrack::Content,
            alignment: ContentAlignment::Start,
        }
    }

    /// Creates a flexible, left-aligned column.
    #[must_use]
    pub const fn flexible(weight: usize) -> Self {
        Self {
            track: GridTrack::Flexible(weight),
            alignment: ContentAlignment::Start,
        }
    }

    /// Sets the horizontal alignment of content within the column.
    #[must_use]
    pub const fn alignment(mut self, alignment: ContentAlignment) -> Self {
        self.alignment = alignment;
        self
    }
}

/// A two-dimensional layout with shared column widths and row heights.
pub struct Grid {
    rows: Vec<Vec<Box<dyn View>>>,
    columns: Vec<GridColumn>,
    column_gap: usize,
    row_gap: usize,
    gap_char: Option<char>,
    intrinsic_widths: Vec<usize>,
    cell_sizes: Vec<Vec<Size>>,
    row_heights: Vec<usize>,
    column_widths: Vec<usize>,
    origin: (usize, usize),
    view_style: ViewStyle,
    bounds: Option<Rect>,
}

impl Grid {
    /// Creates a grid from rows of views.
    #[must_use]
    pub fn new(rows: Vec<Vec<Box<dyn View>>>) -> Self {
        Self {
            rows,
            columns: Vec::new(),
            column_gap: 0,
            row_gap: 0,
            gap_char: None,
            intrinsic_widths: Vec::new(),
            cell_sizes: Vec::new(),
            row_heights: Vec::new(),
            column_widths: Vec::new(),
            origin: (0, 0),
            view_style: ViewStyle::new(),
            bounds: None,
        }
    }

    /// Sets the sizing behavior for each column.
    #[must_use]
    pub fn columns(mut self, columns: impl IntoIterator<Item = GridColumn>) -> Self {
        self.columns = columns.into_iter().collect();
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

    /// Sets the character used to render row and column gaps.
    #[must_use]
    pub const fn gap_char(mut self, ch: char) -> Self {
        self.gap_char = Some(ch);
        self
    }

    fn column_count(&self) -> usize {
        self.rows.iter().map(Vec::len).max().unwrap_or(0)
    }

    fn track(&self, column: usize) -> GridTrack {
        self.columns
            .get(column)
            .map_or(GridTrack::Content, |column| column.track)
    }

    fn alignment(&self, column: usize) -> ContentAlignment {
        self.columns
            .get(column)
            .map_or(ContentAlignment::Start, |column| column.alignment)
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
                let all_flexible = (0..widths.len()).all(|column| {
                    matches!(self.track(column), GridTrack::Flexible(weight) if weight > 0)
                });
                if all_flexible {
                    let track_space = available.saturating_sub(gaps);
                    let mut distributed = 0usize;
                    for (column, width) in widths.iter_mut().enumerate() {
                        let track_weight = match self.track(column) {
                            GridTrack::Flexible(track_weight) => track_weight,
                            GridTrack::Content => 0,
                        };
                        let allocation = track_space
                            .saturating_mul(track_weight)
                            .checked_div(weight)
                            .unwrap_or_default();
                        *width = allocation;
                        distributed = distributed.saturating_add(allocation);
                    }
                    let remainder = track_space.saturating_sub(distributed);
                    if remainder > 0 {
                        widths[0] = widths[0].saturating_add(remainder);
                    }
                } else {
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
            }
        } else if available < minimum {
            let mut excess = minimum - available;
            for flexible in [true, false] {
                for (column, width) in widths.iter_mut().enumerate().rev() {
                    let is_flexible =
                        matches!(self.track(column), GridTrack::Flexible(weight) if weight > 0);
                    if is_flexible != flexible {
                        continue;
                    }
                    let reduction = (*width).min(excess);
                    *width -= reduction;
                    excess -= reduction;
                    if excess == 0 {
                        break;
                    }
                }
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

    fn render_gap(&self, buffer: &mut Buffer, gap_char: char) {
        let (origin_x, origin_y) = self.origin;
        let total_width = self.total_size(&self.column_widths, &[]).width;
        let mut y = origin_y;

        for (row_index, row_height) in self.row_heights.iter().copied().enumerate() {
            let mut x = origin_x;
            for (column, column_width) in self.column_widths.iter().copied().enumerate() {
                x = x.saturating_add(column_width);
                if column + 1 < self.column_widths.len() {
                    for gap_x in 0..self.column_gap {
                        for gap_y in 0..row_height {
                            if let Some(cell) = buffer.cell_mut(x + gap_x, y + gap_y) {
                                cell.ch = gap_char;
                            }
                        }
                    }
                }
                x = x.saturating_add(self.column_gap);
            }

            if row_index + 1 < self.row_heights.len() {
                for gap_y in 0..self.row_gap {
                    for gap_x in 0..total_width {
                        if let Some(cell) =
                            buffer.cell_mut(origin_x + gap_x, y + row_height + gap_y)
                        {
                            cell.ch = gap_char;
                        }
                    }
                }
            }
            y = y.saturating_add(row_height).saturating_add(self.row_gap);
        }
    }

    fn render_children(&self, buffer: &mut Buffer) {
        for row in &self.rows {
            for cell in row {
                cell.render(buffer);
            }
        }
    }
}

impl View for Grid {
    fn measure(&mut self, constraints: Constraints) -> Size {
        let outer_constraints = self.view_style.resolve(constraints);
        let content_constraints = self.view_style.content_constraints(constraints);
        let column_count = self.column_count();
        let mut intrinsic_widths = vec![0; column_count];
        let mut row_heights = Vec::with_capacity(self.rows.len());
        let mut cell_sizes = Vec::with_capacity(self.rows.len());

        for row in &mut self.rows {
            let mut row_height = 0;
            let mut row_sizes = Vec::with_capacity(row.len());
            for (column, cell) in row.iter_mut().enumerate() {
                let size = cell.measure(Constraints::at_most(
                    usize::MAX,
                    content_constraints.height.max,
                ));
                intrinsic_widths[column] = intrinsic_widths[column].max(size.width);
                row_height = row_height.max(size.height);
                row_sizes.push(size);
            }
            cell_sizes.push(row_sizes);
            row_heights.push(row_height);
        }

        self.intrinsic_widths = intrinsic_widths;
        self.row_heights = row_heights;
        self.column_widths =
            self.resolve_widths(content_constraints.width.max, &self.intrinsic_widths);

        for (row_index, row) in self.rows.iter_mut().enumerate() {
            for (column, cell) in row.iter_mut().enumerate() {
                cell_sizes[row_index][column] = cell.measure(Constraints::at_most(
                    self.column_widths[column],
                    self.row_heights[row_index],
                ));
            }
        }

        self.cell_sizes = cell_sizes;

        outer_constraints.clamp(
            self.view_style
                .outer_size(self.total_size(&self.column_widths, &self.row_heights)),
        )
    }

    fn style(&self) -> &ViewStyle {
        &self.view_style
    }

    fn style_mut(&mut self) -> &mut ViewStyle {
        &mut self.view_style
    }

    fn arrange(&mut self, bounds: Rect) {
        self.bounds = Some(bounds);
        let geometry = self.view_style.geometry(bounds);
        self.origin = (geometry.content.x, geometry.content.y);
        self.column_widths = self.resolve_widths(geometry.content.width, &self.intrinsic_widths);
        let alignments: Vec<_> = (0..self.column_widths.len())
            .map(|column| self.alignment(column))
            .collect();
        let mut y = geometry.content.y;
        for (row_index, row) in self.rows.iter_mut().enumerate() {
            let mut x = geometry.content.x;
            for (column, cell) in row.iter_mut().enumerate() {
                let column_width = self.column_widths[column];
                let cell_width = self.cell_sizes[row_index][column].width.min(column_width);
                let offset = alignments[column].offset(column_width, cell_width);
                cell.arrange(Rect::new(
                    x.saturating_add(offset),
                    y,
                    cell_width,
                    self.row_heights[row_index],
                ));
                x = x
                    .saturating_add(column_width)
                    .saturating_add(self.column_gap);
            }
            y = y
                .saturating_add(self.row_heights[row_index])
                .saturating_add(self.row_gap);
        }
    }

    fn render(&self, buffer: &mut Buffer) {
        if let Some(bounds) = self.bounds {
            self.view_style
                .render_decorations(buffer, self.view_style.geometry(bounds));
        }
        if let Some(gap_char) = self.gap_char {
            self.render_gap(buffer, gap_char);
        }
        self.render_children(buffer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::render;
    use crate::{BorderStyle, Span, ViewStyleExt};

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
                Span::new("3x3 gap 1"),
                text_grid(&[
                    &["A", "B", "C"][..],
                    &["D", "E", "F"][..],
                    &["G", "H", "I"][..],
                ])
                .column_gap(1)
                .row_gap(1)
                .gap_char('░')
                .border(BorderStyle::default()),
            ]),
            Box::new(crate::vstack![
                Span::new("3x3 content columns fit content"),
                text_grid(&[
                    &["AAA", "b", "c"][..],
                    &["d", "BBBB", "f"][..],
                    &["g", "h", "CCCCC"][..],
                ])
                .columns([
                    GridColumn::content(),
                    GridColumn::content(),
                    GridColumn::content(),
                ])
                .column_gap(1)
                .row_gap(1)
                .gap_char('░')
                .border(BorderStyle::default()),
            ]),
            Box::new(crate::vstack![
                Span::new("3x3 middle column flex"),
                text_grid(&[
                    &["A", "middle", "X"][..],
                    &["BB", "center", "YYY"][..],
                    &["C", "wide", "Z"][..],
                ])
                .columns([
                    GridColumn::content(),
                    GridColumn::flexible(1),
                    GridColumn::content(),
                ])
                .column_gap(1)
                .gap_char('░')
                .border(BorderStyle::default())
                .width(22),
            ]),
            Box::new(crate::vstack![
                Span::new("3x3 flex columns 1,2,1"),
                text_grid(&[
                    &["A", "B", "C"][..],
                    &["DD", "EEE", "F"][..],
                    &["G", "H", "II"][..],
                ])
                .columns([
                    GridColumn::flexible(1),
                    GridColumn::flexible(2),
                    GridColumn::flexible(1),
                ])
                .column_gap(1)
                .row_gap(1)
                .gap_char('░')
                .border(BorderStyle::default())
                .width(22),
            ]),
            Box::new(crate::vstack![
                Span::new("3x3 flex columns 1,2,1 gap 2"),
                text_grid(&[
                    &["A", "B", "C"][..],
                    &["DD", "EEE", "F"][..],
                    &["G", "H", "II"][..],
                ])
                .columns([
                    GridColumn::flexible(1),
                    GridColumn::flexible(2),
                    GridColumn::flexible(1),
                ])
                .column_gap(2)
                .row_gap(2)
                .gap_char('░')
                .border(BorderStyle::default())
                .width(22),
            ]),
        ];
        insta::assert_snapshot!(render(crate::vstack![..cases].gap(1)));
    }

    #[test]
    fn snapshots_grid_alignment_cases() {
        let grid = text_grid(&[&["left", "center", "right"][..], &["L", "C", "R"][..]])
            .columns([
                GridColumn::flexible(1),
                GridColumn::flexible(1).alignment(ContentAlignment::Center),
                GridColumn::flexible(1).alignment(ContentAlignment::End),
            ])
            .column_gap(1)
            .row_gap(1)
            .gap_char('░');

        insta::assert_snapshot!(render(crate::vstack![
            Span::new("left / center / right"),
            grid.width(35).border(BorderStyle::default()),
        ]));
    }
}
