use std::fmt::Write;

pub struct Buffer {
    cells: Vec<Vec<Cell>>,
}

impl Buffer {
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![vec![Cell::blank(); width]; height],
        }
    }

    #[must_use]
    pub fn width(&self) -> usize {
        self.cells.first().map_or(0, Vec::len)
    }

    #[must_use]
    pub const fn height(&self) -> usize {
        self.cells.len()
    }

    #[must_use]
    pub fn cell(&self, x: usize, y: usize) -> Option<&Cell> {
        self.cells.get(y).and_then(|row| row.get(x))
    }

    #[must_use]
    pub fn cell_mut(&mut self, x: usize, y: usize) -> Option<&mut Cell> {
        self.cells.get_mut(y).and_then(|row| row.get_mut(x))
    }

    #[must_use]
    pub fn to_plain(&self) -> String {
        self.cells
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| cell.ch)
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[must_use]
    pub fn to_ansi(&self) -> String {
        self.cells
            .iter()
            .map(|row| Self::row_to_ansi(row))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn row_to_ansi(row: &[Cell]) -> String {
        let content_end = row
            .iter()
            .rposition(|cell| cell.ch != ' ' || cell.style != CellStyle::default())
            .map_or(0, |last| last + 1);

        let mut out = String::new();
        let mut active = CellStyle::default();
        for cell in &row[..content_end] {
            cell.style.write_delta(active, &mut out);
            active = cell.style;
            out.push(cell.ch);
        }
        if active != CellStyle::default() {
            out.push_str("\u{1b}[0m");
        }
        out
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct CellStyle {
    fg: Option<Color>,
    bg: Option<Color>,
    bold: bool,
    underline: bool,
    italic: bool,
}

impl CellStyle {
    #[must_use]
    pub const fn fg(color: Color) -> Self {
        Self {
            fg: Some(color),
            bg: None,
            bold: false,
            underline: false,
            italic: false,
        }
    }

    #[must_use]
    pub const fn with_fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    #[must_use]
    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    #[must_use]
    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    #[must_use]
    pub const fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    #[must_use]
    pub const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    fn write_delta(self, previous: Self, out: &mut String) {
        let mut codes = Vec::new();
        if self.fg != previous.fg {
            codes.push(self.fg.map_or(39, Color::fg_code));
        }
        if self.bg != previous.bg {
            codes.push(self.bg.map_or(49, Color::bg_code));
        }
        if self.bold != previous.bold {
            codes.push(if self.bold { 1 } else { 22 });
        }
        if self.italic != previous.italic {
            codes.push(if self.italic { 3 } else { 23 });
        }
        if self.underline != previous.underline {
            codes.push(if self.underline { 4 } else { 24 });
        }
        if codes.is_empty() {
            return;
        }

        out.push_str("\u{1b}[");
        for (index, code) in codes.iter().enumerate() {
            if index > 0 {
                out.push(';');
            }
            write!(out, "{code}").expect("writing to a String cannot fail");
        }
        out.push('m');
    }
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Color {
    GRAY,
    RED,
    GREEN,
    CYAN,
    YELLOW,
    WHITE,
    BLACK,
}

impl Color {
    const fn fg_code(self) -> u8 {
        match self {
            Self::RED => 31,
            Self::GREEN => 32,
            Self::CYAN => 36,
            Self::YELLOW => 33,
            Self::GRAY => 90,
            Self::WHITE => 37,
            Self::BLACK => 30,
        }
    }

    const fn bg_code(self) -> u8 {
        match self {
            Self::RED => 41,
            Self::GREEN => 42,
            Self::CYAN => 46,
            Self::YELLOW => 43,
            Self::GRAY => 100,
            Self::WHITE => 47,
            Self::BLACK => 40,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Cell {
    pub ch: char,
    pub style: CellStyle,
}

impl Cell {
    #[must_use]
    pub fn blank() -> Self {
        Self {
            ch: ' ',
            style: CellStyle::default(),
        }
    }
}
