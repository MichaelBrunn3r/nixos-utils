use crate::buffer::{Buffer, CellStyle, Color};
use crate::style::ViewStyle;
use crate::view::{Constraints, Rect, Size, View};

pub struct Text {
    runs: Vec<TextRun>,
    view_style: ViewStyle,
    bounds: Option<Rect>,
}

#[derive(Clone)]
pub struct TextRun {
    text: String,
    style: CellStyle,
}

pub trait TextExt: Sized {
    fn styled(self, style: CellStyle) -> TextRun;

    fn bold(self) -> TextRun {
        self.styled(CellStyle::default().bold())
    }

    fn underline(self) -> TextRun {
        self.styled(CellStyle::default().underline())
    }

    fn italic(self) -> TextRun {
        self.styled(CellStyle::default().italic())
    }

    fn fg(self, color: Color) -> TextRun {
        self.styled(CellStyle::fg(color))
    }
}

impl TextExt for &str {
    fn styled(self, style: CellStyle) -> TextRun {
        TextRun {
            text: self.to_owned(),
            style,
        }
    }
}

impl TextExt for String {
    fn styled(self, style: CellStyle) -> TextRun {
        TextRun { text: self, style }
    }
}

impl TextExt for TextRun {
    fn styled(mut self, style: CellStyle) -> TextRun {
        self.style = style;
        self
    }

    fn bold(mut self) -> TextRun {
        self.style = self.style.bold();
        self
    }

    fn underline(mut self) -> TextRun {
        self.style = self.style.underline();
        self
    }

    fn italic(mut self) -> TextRun {
        self.style = self.style.italic();
        self
    }

    fn fg(mut self, color: Color) -> TextRun {
        self.style = self.style.with_fg(color);
        self
    }
}

impl From<&str> for TextRun {
    fn from(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            style: CellStyle::default(),
        }
    }
}

impl From<String> for TextRun {
    fn from(text: String) -> Self {
        Self {
            text,
            style: CellStyle::default(),
        }
    }
}

impl Text {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            runs: Vec::new(),
            view_style: ViewStyle::new(),
            bounds: None,
        }
    }

    #[must_use]
    pub fn new(data: impl Into<String>) -> Self {
        Self::empty().push(data.into())
    }

    #[must_use]
    pub fn push(mut self, run: impl Into<TextRun>) -> Self {
        self.runs.push(run.into());
        self
    }

    fn lines(&self, width: usize) -> Vec<Vec<(char, CellStyle)>> {
        if width == 0 {
            return Vec::new();
        }

        let mut lines = vec![Vec::new()];
        for run in &self.runs {
            for ch in run.text.chars() {
                if ch == '\n' {
                    lines.push(Vec::new());
                    continue;
                }
                lines
                    .last_mut()
                    .expect("text always has a line")
                    .push((ch, run.style));
                Self::split_overflow(&mut lines, width);
            }
        }
        if lines.len() == 1 && lines[0].is_empty() {
            Vec::new()
        } else {
            lines
        }
    }

    fn split_overflow(lines: &mut Vec<Vec<(char, CellStyle)>>, width: usize) {
        let line = lines.last_mut().expect("text always has a line");
        if line.len() <= width {
            return;
        }

        let break_at = line
            .iter()
            .rposition(|(ch, _)| *ch == ' ' || *ch == '-')
            .filter(|index| *index < width)
            .map_or(width, |index| index + 1);
        let remainder = line.split_off(break_at);
        if line.last().is_some_and(|(ch, _)| *ch == ' ') {
            line.pop();
        }
        lines.push(remainder);
    }
}

impl View for Text {
    fn measure(&mut self, constraints: Constraints) -> Size {
        let outer_constraints = self.view_style.resolve(constraints);
        let content_constraints = self.view_style.content_constraints(constraints);
        if content_constraints.width.max == 0 || content_constraints.height.max == 0 {
            return Size::new(0, 0);
        }
        let lines = self.lines(content_constraints.width.max);
        let width = lines.iter().map(Vec::len).max().unwrap_or_default();
        let height = lines.len().min(content_constraints.height.max);
        outer_constraints.clamp(self.view_style.outer_size(Size::new(width, height)))
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
        for (y, line) in self.lines(geometry.content.width).into_iter().enumerate() {
            if y >= geometry.content.height {
                break;
            }
            for (x, (ch, style)) in line.into_iter().enumerate() {
                if let Some(cell) = buffer.cell_mut(
                    geometry.content.x.saturating_add(x),
                    geometry.content.y.saturating_add(y),
                ) {
                    cell.ch = ch;
                    cell.style = style;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::{Buffer, Color, Constraints, Rect, TextExt, View};

    #[test]
    fn wraps_at_spaces_and_hyphens() {
        let mut text = crate::text!["The quick brown-fox jumps"];
        let width = 12;
        let size = text.measure(Constraints::at_most(width, 10));
        let mut buffer = Buffer::new(width, size.height);
        text.arrange(Rect::new(0, 0, width, size.height));
        text.render(&mut buffer);

        assert_snapshot!(buffer.to_plain());
    }

    #[test]
    fn styled_run_continues_across_wrapped_lines() {
        let mut text = crate::text!["The ", "quick brown".fg(Color::RED), " fox"];
        let size = text.measure(crate::Constraints::at_most(10, 10));
        let mut buffer = crate::Buffer::new(size.width, size.height);
        text.arrange(crate::Rect::new(0, 0, size.width, size.height));
        text.render(&mut buffer);

        assert_snapshot!(buffer.to_ansi());
    }
}
