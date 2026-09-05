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
            .rposition(|cell| cell.ch != ' ' || cell.style.sgr_prefix().is_some())
            .map_or(0, |last| last + 1);

        let mut out = String::new();
        let mut active = None;
        for cell in &row[..content_end] {
            let prefix = cell.style.sgr_prefix();
            if prefix != active {
                if active.is_some() {
                    out.push_str("\u{1b}[0m");
                }
                if let Some(prefix) = prefix {
                    prefix.write_to(&mut out);
                }
                active = prefix;
            }
            out.push(cell.ch);
        }
        if active.is_some() {
            out.push_str("\u{1b}[0m");
        }
        out
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
struct AnsiStyle {
    fg: Option<u8>,
    bg: Option<u8>,
}

impl AnsiStyle {
    fn write_to(self, out: &mut String) {
        if self.fg.is_none() && self.bg.is_none() {
            return;
        }

        out.push_str("\u{1b}[");
        if let Some(fg) = self.fg {
            write!(out, "{fg}").expect("writing to a String cannot fail");
            if self.bg.is_some() {
                out.push(';');
            }
        }
        if let Some(bg) = self.bg {
            write!(out, "{bg}").expect("writing to a String cannot fail");
        }
        out.push('m');
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Style {
    fg: Option<Color>,
    bg: Option<Color>,
}

impl Style {
    #[must_use]
    pub const fn fg(color: Color) -> Self {
        Self {
            fg: Some(color),
            bg: None,
        }
    }

    #[must_use]
    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    #[must_use]
    fn sgr_prefix(self) -> Option<AnsiStyle> {
        match (self.fg, self.bg) {
            (Some(fg), bg) => Some(AnsiStyle {
                fg: Some(fg.fg_code()),
                bg: bg.map(Color::bg_code),
            }),
            (None, Some(bg)) => Some(AnsiStyle {
                fg: None,
                bg: Some(bg.bg_code()),
            }),
            (None, None) => None,
        }
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
    pub style: Style,
}

impl Cell {
    #[must_use]
    pub fn blank() -> Self {
        Self {
            ch: ' ',
            style: Style::default(),
        }
    }
}
