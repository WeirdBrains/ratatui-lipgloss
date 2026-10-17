//! Layout helpers: measuring, joining and placing blocks of text.

use ratatui::style::Style as RStyle;
use ratatui::text::Text;

use crate::Position;
use crate::cells::{
    Row, align_row, align_rows_vertical, block_width, rows_from_text, rows_to_text,
};
use crate::style::DEFAULT_TAB_WIDTH;

fn rows(text: &Text<'_>) -> Vec<Row> {
    rows_from_text(text, RStyle::default(), DEFAULT_TAB_WIDTH)
}

/// The width in columns of the widest line of `text` (Lip Gloss `Width`).
#[must_use]
pub fn width(text: &Text<'_>) -> usize {
    text.lines
        .iter()
        .map(ratatui::text::Line::width)
        .max()
        .unwrap_or(0)
}

/// The number of lines in `text` (Lip Gloss `Height`).
#[must_use]
pub fn height(text: &Text<'_>) -> usize {
    text.lines.len()
}

/// `(width, height)` of `text` (Lip Gloss `Size`).
#[must_use]
pub fn size(text: &Text<'_>) -> (usize, usize) {
    (width(text), height(text))
}

/// Joins blocks side by side. Shorter blocks are aligned vertically at
/// `pos` (`Position::TOP`, `Position::CENTER`, `Position::BOTTOM` or any
/// fraction), and every block is padded to its own width.
///
/// ```
/// use ratatui_lipgloss::{join_horizontal, to_plain_string, Position};
///
/// let t = join_horizontal(Position::BOTTOM, ["a\nb\nc", "x"]);
/// assert_eq!(to_plain_string(&t), "a \nb \ncx");
/// ```
#[must_use]
pub fn join_horizontal<'a, I, T>(pos: impl Into<Position>, blocks: I) -> Text<'static>
where
    I: IntoIterator<Item = T>,
    T: Into<Text<'a>>,
{
    let pos = pos.into();
    let blocks: Vec<Vec<Row>> = blocks.into_iter().map(|b| rows(&b.into())).collect();
    if blocks.is_empty() {
        return Text::default();
    }
    let max_h = blocks.iter().map(Vec::len).max().unwrap_or(0);
    let mut out: Vec<Row> = vec![Vec::new(); max_h];
    for block in blocks {
        let w = block_width(&block);
        let block: Vec<Row> = block
            .into_iter()
            .map(|r| align_row(r, w, Position::LEFT, " ", RStyle::default()))
            .collect();
        let block = align_rows_vertical(block, max_h, pos, w, " ", RStyle::default());
        for (row, part) in out.iter_mut().zip(block) {
            row.extend(part);
        }
    }
    rows_to_text(out)
}

/// Stacks blocks on top of each other. Narrower lines are aligned
/// horizontally at `pos` (`Position::LEFT`, `Position::CENTER`,
/// `Position::RIGHT` or any fraction).
///
/// ```
/// use ratatui_lipgloss::{join_vertical, to_plain_string, Position};
///
/// let t = join_vertical(Position::CENTER, ["abc", "x"]);
/// assert_eq!(to_plain_string(&t), "abc\n x ");
/// ```
#[must_use]
pub fn join_vertical<'a, I, T>(pos: impl Into<Position>, blocks: I) -> Text<'static>
where
    I: IntoIterator<Item = T>,
    T: Into<Text<'a>>,
{
    let pos = pos.into();
    let all: Vec<Row> = blocks.into_iter().flat_map(|b| rows(&b.into())).collect();
    if all.is_empty() {
        return Text::default();
    }
    let w = block_width(&all);
    rows_to_text(
        all.into_iter()
            .map(|r| align_row(r, w, pos, " ", RStyle::default()))
            .collect(),
    )
}

/// How the whitespace added by [`place_with`] looks.
///
/// ```
/// use ratatui::style::{Color, Style};
/// use ratatui_lipgloss::{place_with, to_plain_string, Position, Whitespace};
///
/// let ws = Whitespace::new().chars("·").style(Style::new().fg(Color::DarkGray));
/// let t = place_with(5, 1, Position::CENTER, Position::CENTER, "x", &ws);
/// assert_eq!(to_plain_string(&t), "··x··");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Whitespace {
    style: RStyle,
    chars: String,
}

impl Default for Whitespace {
    fn default() -> Self {
        Self {
            style: RStyle::default(),
            chars: " ".to_string(),
        }
    }
}

impl Whitespace {
    /// Plain, unstyled spaces.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the style of the whitespace (accepts a ratatui style or a
    /// `&ratatui_lipgloss::Style`).
    #[must_use]
    pub fn style(mut self, style: impl Into<RStyle>) -> Self {
        self.style = style.into();
        self
    }

    /// Sets the characters used to fill the whitespace (cycled as a
    /// pattern).
    #[must_use]
    pub fn chars(mut self, chars: impl Into<String>) -> Self {
        let c = chars.into();
        self.chars = if c.is_empty() { " ".to_string() } else { c };
        self
    }
}

/// Places `content` in a `width` × `height` box of whitespace, at the given
/// horizontal and vertical positions. Content larger than the box is left
/// untouched on that axis.
///
/// ```
/// use ratatui_lipgloss::{place, to_plain_string, Position};
///
/// let t = place(4, 3, Position::RIGHT, Position::BOTTOM, "x");
/// assert_eq!(to_plain_string(&t), "    \n    \n   x");
/// ```
#[must_use]
pub fn place<'a>(
    width: u16,
    height: u16,
    horizontal: impl Into<Position>,
    vertical: impl Into<Position>,
    content: impl Into<Text<'a>>,
) -> Text<'static> {
    place_with(
        width,
        height,
        horizontal,
        vertical,
        content,
        &Whitespace::default(),
    )
}

/// Like [`place`], with styled / patterned whitespace.
#[must_use]
pub fn place_with<'a>(
    width: u16,
    height: u16,
    horizontal: impl Into<Position>,
    vertical: impl Into<Position>,
    content: impl Into<Text<'a>>,
    whitespace: &Whitespace,
) -> Text<'static> {
    let (h, v) = (horizontal.into(), vertical.into());
    let rows = rows(&content.into());
    let w = block_width(&rows).max(usize::from(width));
    let rows: Vec<Row> = rows
        .into_iter()
        .map(|r| align_row(r, w, h, &whitespace.chars, whitespace.style))
        .collect();
    let rows = align_rows_vertical(
        rows,
        usize::from(height),
        v,
        w,
        &whitespace.chars,
        whitespace.style,
    );
    rows_to_text(rows)
}

/// Places `content` horizontally in a space `width` columns wide.
#[must_use]
pub fn place_horizontal<'a>(
    width: u16,
    pos: impl Into<Position>,
    content: impl Into<Text<'a>>,
) -> Text<'static> {
    place(width, 0, pos, Position::TOP, content)
}

/// Places `content` vertically in a space `height` rows tall.
#[must_use]
pub fn place_vertical<'a>(
    height: u16,
    pos: impl Into<Position>,
    content: impl Into<Text<'a>>,
) -> Text<'static> {
    place(0, height, Position::LEFT, pos, content)
}
