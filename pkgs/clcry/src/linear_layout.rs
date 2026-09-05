use crate::buffer::Buffer;
use crate::direction::Direction;
use crate::style::ViewStyle;
use crate::view::{Constraints, Rect, Size, View};

/// A linear container parameterized by its line-placement policy.
pub struct LinearLayout<S> {
    direction: Direction,
    strategy: S,
    content_alignment: ContentAlignment,
    children: Vec<Box<dyn View>>,
    child_sizes: Vec<Size>,
    placements: Vec<Rect>,
    gap: usize,
    view_style: ViewStyle,
    bounds: Option<Rect>,
}

impl<S: LayoutStrategy> LinearLayout<S> {
    /// Creates a layout with the given direction and children.
    #[must_use]
    pub fn new(direction: Direction, children: Vec<Box<dyn View>>) -> Self {
        Self {
            direction,
            strategy: S::default(),
            content_alignment: ContentAlignment::Start,
            children,
            child_sizes: Vec::new(),
            placements: Vec::new(),
            gap: 0,
            view_style: ViewStyle::new(),
            bounds: None,
        }
    }

    /// Sets the space between children and wrapped lines.
    #[must_use]
    pub const fn gap(mut self, gap: usize) -> Self {
        self.gap = gap;
        self
    }

    /// Aligns the content along the layout's main axis.
    #[must_use]
    pub const fn content_alignment(mut self, alignment: ContentAlignment) -> Self {
        self.content_alignment = alignment;
        self
    }

    /// Returns the available size supplied by the parent constraints.
    const fn available_size(constraints: Constraints) -> Size {
        Size::new(constraints.width.max, constraints.height.max)
    }

    /// Computes child placements and the space occupied by those placements.
    fn compute_layout(&self, available: Size, child_sizes: &[Size]) -> (Vec<Rect>, Size) {
        let available_parallel = available.extent_parallel_to(self.direction);
        let available_perpendicular = available.extent_perpendicular_to(self.direction);
        let mut placements = Vec::with_capacity(child_sizes.len());
        let mut parallel = 0usize;
        let mut perpendicular = 0usize;
        let mut line_perpendicular = 1usize;
        let mut used_parallel = 0usize;

        for child_size in child_sizes {
            let child_parallel = child_size.extent_parallel_to(self.direction);
            let next_parallel = if parallel == 0 {
                child_parallel
            } else {
                parallel
                    .saturating_add(self.gap)
                    .saturating_add(child_parallel)
            };

            if self.strategy.wraps() && parallel != 0 && next_parallel > available_parallel {
                used_parallel = used_parallel.max(parallel);
                parallel = 0;
                perpendicular = perpendicular
                    .saturating_add(line_perpendicular)
                    .saturating_add(self.gap);
                line_perpendicular = 1;
            }

            if perpendicular >= available_perpendicular {
                break;
            }

            if parallel != 0 {
                parallel = parallel.saturating_add(self.gap);
            }
            let placed_parallel = child_parallel.min(available_parallel.saturating_sub(parallel));
            let placed_perpendicular = child_size
                .extent_perpendicular_to(self.direction)
                .min(available_perpendicular.saturating_sub(perpendicular));
            placements.push(self.direction.rect(
                parallel,
                perpendicular,
                self.direction.size(placed_parallel, placed_perpendicular),
            ));
            parallel = parallel.saturating_add(placed_parallel);
            line_perpendicular = line_perpendicular.max(placed_perpendicular);
        }

        used_parallel = used_parallel.max(parallel);
        let used_perpendicular = if placements.is_empty() {
            0
        } else {
            perpendicular
                .saturating_add(line_perpendicular)
                .min(available_perpendicular)
        };
        (
            placements,
            self.direction
                .size(used_parallel.min(available_parallel), used_perpendicular),
        )
    }

    #[allow(clippy::too_many_lines)]
    fn resolve_flex_sizes(&mut self, available: Size) {
        if self.strategy.wraps() {
            return;
        }

        let available_parallel = available.extent_parallel_to(self.direction);
        let gap_width = self
            .gap
            .saturating_mul(self.child_sizes.len().saturating_sub(1));
        let fixed_parallel = self
            .child_sizes
            .iter()
            .zip(&self.children)
            .filter(|(_, child)| child.style().flex_grow() == 0)
            .map(|(size, _)| size.extent_parallel_to(self.direction))
            .sum::<usize>()
            .saturating_add(gap_width);
        let flexible_parallel = self
            .child_sizes
            .iter()
            .zip(&self.children)
            .filter(|(_, child)| child.style().flex_grow() > 0)
            .map(|(size, _)| size.extent_parallel_to(self.direction))
            .sum::<usize>();
        let growth = self
            .children
            .iter()
            .map(|child| child.style().flex_grow())
            .sum::<usize>();
        if growth == 0 {
            return;
        }

        let available_flexible = available_parallel.saturating_sub(fixed_parallel);
        if available_flexible >= flexible_parallel {
            let spare = available_flexible - flexible_parallel;
            let mut distributed = 0usize;
            for (child, size) in self.children.iter().zip(&mut self.child_sizes) {
                let extra = if spare == 0 {
                    0
                } else {
                    spare.saturating_mul(child.style().flex_grow()) / growth
                };
                distributed = distributed.saturating_add(extra);
                let parallel = size
                    .extent_parallel_to(self.direction)
                    .saturating_add(extra);
                *size = self
                    .direction
                    .size(parallel, size.extent_perpendicular_to(self.direction));
            }
            let remainder = spare.saturating_sub(distributed);
            if remainder > 0
                && let Some(size) = self
                    .child_sizes
                    .iter_mut()
                    .zip(&self.children)
                    .find(|(_, child)| child.style().flex_grow() > 0)
                    .map(|(size, _)| size)
            {
                let parallel = size
                    .extent_parallel_to(self.direction)
                    .saturating_add(remainder);
                *size = self
                    .direction
                    .size(parallel, size.extent_perpendicular_to(self.direction));
            }
        } else {
            let mut allocated = 0usize;
            for (child, size) in self.children.iter().zip(&mut self.child_sizes) {
                if child.style().flex_grow() == 0 {
                    continue;
                }
                let parallel = if flexible_parallel == 0 {
                    0
                } else {
                    size.extent_parallel_to(self.direction)
                        .saturating_mul(available_flexible)
                        .checked_div(flexible_parallel)
                        .unwrap_or_default()
                };
                allocated = allocated.saturating_add(parallel);
                *size = self
                    .direction
                    .size(parallel, size.extent_perpendicular_to(self.direction));
            }
            let remainder = available_flexible.saturating_sub(allocated);
            if remainder > 0
                && let Some(size) = self
                    .child_sizes
                    .iter_mut()
                    .zip(&self.children)
                    .find(|(_, child)| child.style().flex_grow() > 0)
                    .map(|(size, _)| size)
            {
                let parallel = size
                    .extent_parallel_to(self.direction)
                    .saturating_add(remainder);
                *size = self
                    .direction
                    .size(parallel, size.extent_perpendicular_to(self.direction));
            }
        }
    }
}

impl<S: LayoutStrategy> View for LinearLayout<S> {
    fn measure(&mut self, constraints: Constraints) -> Size {
        let outer_constraints = self.view_style.resolve(constraints);
        let content_constraints = self.view_style.content_constraints(constraints);
        let available = Self::available_size(content_constraints);
        let has_flex = !self.strategy.wraps()
            && self
                .children
                .iter()
                .any(|child| child.style().flex_grow() > 0);
        let mut child_sizes = Vec::with_capacity(self.children.len());
        let mut used_parallel = 0usize;

        for (index, child) in self.children.iter_mut().enumerate() {
            if index > 0 && !self.strategy.wraps() && !has_flex {
                used_parallel = used_parallel.saturating_add(self.gap);
            }
            let child_size = if has_flex {
                child.measure(self.direction.constraints(
                    usize::MAX,
                    available.extent_perpendicular_to(self.direction),
                ))
            } else {
                child.measure(self.strategy.child_constraints(
                    self.direction,
                    available,
                    used_parallel,
                ))
            };
            let child_parallel = child_size.extent_parallel_to(self.direction);
            child_sizes.push(child_size);
            used_parallel = used_parallel.saturating_add(child_parallel);
        }

        self.child_sizes = child_sizes;
        self.resolve_flex_sizes(available);
        if has_flex {
            for (child, size) in self.children.iter_mut().zip(&self.child_sizes) {
                if child.style().flex_grow() > 0 {
                    let parallel = size.extent_parallel_to(self.direction);
                    child.measure(
                        self.direction.constraints(
                            parallel,
                            available.extent_perpendicular_to(self.direction),
                        ),
                    );
                }
            }
        }
        self.placements.clear();
        outer_constraints.clamp(
            self.view_style
                .outer_size(self.compute_layout(available, &self.child_sizes).1),
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
        let available = Size::new(geometry.content.width, geometry.content.height);
        let (placements, content_size) = self.compute_layout(available, &self.child_sizes);
        let alignment_offset = self.content_alignment.offset(
            available.extent_parallel_to(self.direction),
            content_size.extent_parallel_to(self.direction),
        );
        self.placements = placements
            .into_iter()
            .map(|local| {
                let local = match self.direction {
                    Direction::Row => Rect::new(
                        local.x.saturating_add(alignment_offset),
                        local.y,
                        local.width,
                        local.height,
                    ),
                    Direction::Column => Rect::new(
                        local.x,
                        local.y.saturating_add(alignment_offset),
                        local.width,
                        local.height,
                    ),
                };
                Rect::new(
                    geometry.content.x.saturating_add(local.x),
                    geometry.content.y.saturating_add(local.y),
                    local.width,
                    local.height,
                )
            })
            .collect();

        for (child, placement) in self.children.iter_mut().zip(&self.placements) {
            child.arrange(*placement);
        }
    }

    fn render(&self, buffer: &mut Buffer) {
        if let Some(bounds) = self.bounds {
            self.view_style
                .render_decorations(buffer, self.view_style.geometry(bounds));
        }
        for child in &self.children {
            child.render(buffer);
        }
    }
}

/// Alignment of the complete layout content along its main axis.
#[derive(Debug, Default, Copy, Clone)]
pub enum ContentAlignment {
    /// Place content at the start of the main axis.
    #[default]
    Start,
    /// Center content along the main axis.
    Center,
    /// Place content at the end of the main axis.
    End,
}

impl ContentAlignment {
    pub(crate) const fn offset(self, available: usize, content: usize) -> usize {
        let remaining = available.saturating_sub(content);
        match self {
            Self::Start => 0,
            Self::Center => remaining / 2,
            Self::End => remaining,
        }
    }
}

/// Placement policy for a linear layout.
pub trait LayoutStrategy: Default {
    /// Returns the constraints used when measuring a child.
    fn child_constraints(
        &self,
        direction: Direction,
        available: Size,
        used_parallel: usize,
    ) -> Constraints;

    /// Returns whether children can move to a new line.
    fn wraps(&self) -> bool;
}

/// Places all children on one line.
#[derive(Debug, Default, Copy, Clone)]
pub struct NoWrap;

impl LayoutStrategy for NoWrap {
    fn child_constraints(
        &self,
        direction: Direction,
        available: Size,
        used_parallel: usize,
    ) -> Constraints {
        direction.constraints(
            available
                .extent_parallel_to(direction)
                .saturating_sub(used_parallel),
            available.extent_perpendicular_to(direction),
        )
    }

    fn wraps(&self) -> bool {
        false
    }
}

/// Places children on additional lines when the main axis is full.
#[derive(Debug, Default, Copy, Clone)]
pub struct Wrap;

impl LayoutStrategy for Wrap {
    fn child_constraints(
        &self,
        direction: Direction,
        available: Size,
        _used_parallel: usize,
    ) -> Constraints {
        direction.constraints(
            available.extent_parallel_to(direction),
            available.extent_perpendicular_to(direction),
        )
    }

    fn wraps(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{create_item_spans, render};
    use crate::{BorderStyle, Span, ViewStyleExt};

    #[test]
    fn layout_directions_and_wrapping() {
        let rows: Vec<Box<dyn View>> = [0usize, 1, 4]
            .into_iter()
            .map(|gap| Box::new(crate::hstack![..create_item_spans(4)].gap(gap)) as Box<dyn View>)
            .collect();
        let columns: Vec<Box<dyn View>> = [0usize, 1, 4]
            .into_iter()
            .map(|gap| Box::new(crate::vstack![..create_item_spans(4)].gap(gap)) as Box<dyn View>)
            .collect();
        let layout = crate::vstack![
            Span::new("Wrapping row"),
            crate::hflex![..create_item_spans(10)]
                .border(BorderStyle::default())
                .max_width(17),
            Span::new("Wrapping column"),
            crate::vflex![..create_item_spans(10)]
                .border(BorderStyle::default())
                .max_height(5),
            Span::new("Non-wrapping row"),
            crate::vstack![..rows].border(BorderStyle::default()),
            Span::new("Non-wrapping column"),
            crate::hstack![..columns].border(BorderStyle::default()),
        ];
        insta::assert_snapshot!(render(layout));
    }

    #[test]
    fn content_alignment() {
        fn bordered(direction: Direction, alignment: ContentAlignment) -> impl View {
            LinearLayout::<NoWrap>::new(direction, create_item_spans(2))
                .content_alignment(alignment)
                .border(BorderStyle::default())
                .width(if matches!(direction, Direction::Row) {
                    15
                } else {
                    7
                })
                .height(if matches!(direction, Direction::Row) {
                    3
                } else {
                    6
                })
        }

        let layout = crate::vstack![
            Span::new("Row start"),
            bordered(Direction::Row, ContentAlignment::Start),
            Span::new("Row center"),
            bordered(Direction::Row, ContentAlignment::Center),
            Span::new("Row end"),
            bordered(Direction::Row, ContentAlignment::End),
            Span::new("Column start"),
            bordered(Direction::Column, ContentAlignment::Start),
            Span::new("Column center"),
            bordered(Direction::Column, ContentAlignment::Center),
            Span::new("Column end"),
            bordered(Direction::Column, ContentAlignment::End),
        ];
        insta::assert_snapshot!(render(layout));
    }

    #[test]
    fn styled_children_grow_in_non_wrapping_layouts() {
        let mut layout = crate::hstack![crate::span!("A").flex_grow(1), crate::span!("B"),];

        assert_eq!(layout.measure(Constraints::exact(5, 1)), Size::new(5, 1));
    }

    #[test]
    fn supports_exact_and_minimum_dimensions() {
        let mut exact = crate::hstack![crate::span!("hello")].width(10).height(3);
        assert_eq!(
            exact.measure(Constraints::at_most(20, 20)),
            Size::new(10, 3)
        );

        let mut minimum = crate::hstack![crate::span!("hello")]
            .min_width(12)
            .min_height(2);
        assert_eq!(
            minimum.measure(Constraints::at_most(20, 20)),
            Size::new(12, 2)
        );
    }
}
