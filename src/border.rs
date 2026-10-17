//! Border definitions and presets.

use ratatui::style::Style as RStyle;
use ratatui::symbols::border;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::cells::{Cell, Row, pattern_fill};

/// The characters used to draw a border, like Lip Gloss's `Border` struct.
///
/// Each edge (`top`, `bottom`, `left`, `right`) may contain several
/// graphemes; they are repeated as a pattern along the edge, so
/// `top: "._.:*:"` produces `._.:*:._.:*:...`. The `middle_*` characters are
/// used by table-like layouts.
///
/// Use one of the presets ([`Border::normal`], [`Border::rounded`], ...) or
/// build your own:
///
/// ```
/// use ratatui_lipgloss::Border;
///
/// const CUTE: Border = Border {
///     top: "._.:*:",
///     bottom: "._.:*:",
///     left: "|*",
///     right: "|*",
///     top_left: "*",
///     top_right: "*",
///     bottom_left: "*",
///     bottom_right: "*",
///     ..Border::empty()
/// };
/// ```
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Border {
    pub top: &'static str,
    pub bottom: &'static str,
    pub left: &'static str,
    pub right: &'static str,
    pub top_left: &'static str,
    pub top_right: &'static str,
    pub bottom_left: &'static str,
    pub bottom_right: &'static str,
    pub middle_left: &'static str,
    pub middle_right: &'static str,
    pub middle: &'static str,
    pub middle_top: &'static str,
    pub middle_bottom: &'static str,
}

macro_rules! border_preset {
    ($(#[$doc:meta])* $name:ident, $top:literal, $bottom:literal, $left:literal, $right:literal,
     $tl:literal, $tr:literal, $bl:literal, $br:literal,
     $ml:literal, $mr:literal, $m:literal, $mt:literal, $mb:literal) => {
        $(#[$doc])*
        #[must_use]
        pub const fn $name() -> Self {
            Self {
                top: $top,
                bottom: $bottom,
                left: $left,
                right: $right,
                top_left: $tl,
                top_right: $tr,
                bottom_left: $bl,
                bottom_right: $br,
                middle_left: $ml,
                middle_right: $mr,
                middle: $m,
                middle_top: $mt,
                middle_bottom: $mb,
            }
        }
    };
}

impl Border {
    /// A border with no characters at all.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            top: "",
            bottom: "",
            left: "",
            right: "",
            top_left: "",
            top_right: "",
            bottom_left: "",
            bottom_right: "",
            middle_left: "",
            middle_right: "",
            middle: "",
            middle_top: "",
            middle_bottom: "",
        }
    }

    border_preset!(
        /// A standard square border: `┌─┐ │ └─┘`.
        normal, "─", "─", "│", "│", "┌", "┐", "└", "┘", "├", "┤", "┼", "┬", "┴"
    );
    border_preset!(
        /// A border with rounded corners: `╭─╮ │ ╰─╯`.
        rounded, "─", "─", "│", "│", "╭", "╮", "╰", "╯", "├", "┤", "┼", "┬", "┴"
    );
    border_preset!(
        /// A heavier border: `┏━┓ ┃ ┗━┛`.
        thick, "━", "━", "┃", "┃", "┏", "┓", "┗", "┛", "┣", "┫", "╋", "┳", "┻"
    );
    border_preset!(
        /// A double-line border: `╔═╗ ║ ╚═╝`.
        double, "═", "═", "║", "║", "╔", "╗", "╚", "╝", "╠", "╣", "╬", "╦", "╩"
    );
    border_preset!(
        /// A border made of full blocks: `█`.
        block, "█", "█", "█", "█", "█", "█", "█", "█", "█", "█", "█", "█", "█"
    );
    border_preset!(
        /// A half-block border drawn outside the content.
        outer_half_block, "▀", "▄", "▌", "▐", "▛", "▜", "▙", "▟", "", "", "", "", ""
    );
    border_preset!(
        /// A half-block border drawn inside the content.
        inner_half_block, "▄", "▀", "▐", "▌", "▗", "▖", "▝", "▘", "", "", "", "", ""
    );
    border_preset!(
        /// A border made of spaces: takes up room but is invisible (unless it
        /// has a background color).
        hidden, " ", " ", " ", " ", " ", " ", " ", " ", " ", " ", " ", " ", " "
    );
    border_preset!(
        /// An ASCII border: `+-+ | +-+`.
        ascii, "-", "-", "|", "|", "+", "+", "+", "+", "+", "+", "+", "+", "+"
    );
    border_preset!(
        /// A Markdown-table style border: `|-|`.
        markdown, "-", "-", "|", "|", "|", "|", "|", "|", "|", "|", "|", "|", "|"
    );

    /// Converts to a ratatui [`border::Set`] (for use with ratatui's `Block`).
    ///
    /// Multi-grapheme edge patterns are passed through verbatim; ratatui
    /// uses only the first cell of each.
    #[must_use]
    pub const fn to_border_set(self) -> border::Set<'static> {
        border::Set {
            top_left: self.top_left,
            top_right: self.top_right,
            bottom_left: self.bottom_left,
            bottom_right: self.bottom_right,
            vertical_left: self.left,
            vertical_right: self.right,
            horizontal_top: self.top,
            horizontal_bottom: self.bottom,
        }
    }
}

impl From<border::Set<'static>> for Border {
    fn from(s: border::Set<'static>) -> Self {
        Self {
            top: s.horizontal_top,
            bottom: s.horizontal_bottom,
            left: s.vertical_left,
            right: s.vertical_right,
            top_left: s.top_left,
            top_right: s.top_right,
            bottom_left: s.bottom_left,
            bottom_right: s.bottom_right,
            ..Self::empty()
        }
    }
}

impl From<Border> for border::Set<'static> {
    fn from(b: Border) -> Self {
        b.to_border_set()
    }
}

/// A resolved border ready for drawing.
#[derive(Clone, Debug)]
pub(crate) struct BorderSpec {
    pub border: Border,
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
    pub top_style: RStyle,
    pub right_style: RStyle,
    pub bottom_style: RStyle,
    pub left_style: RStyle,
}

fn or_space(s: &'static str) -> &'static str {
    if s.is_empty() { " " } else { s }
}

fn nth_grapheme(pattern: &'static str, i: usize) -> &'static str {
    let graphemes: Vec<&str> = or_space(pattern).graphemes(true).collect();
    graphemes[i % graphemes.len()]
}

fn max_grapheme_width(pattern: &'static str) -> usize {
    or_space(pattern)
        .graphemes(true)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(1)
}

impl BorderSpec {
    pub fn left_width(&self) -> usize {
        if self.left {
            max_grapheme_width(self.border.left)
        } else {
            0
        }
    }

    pub fn right_width(&self) -> usize {
        if self.right {
            max_grapheme_width(self.border.right)
        } else {
            0
        }
    }

    pub fn horizontal_size(&self) -> usize {
        self.left_width() + self.right_width()
    }

    pub fn vertical_size(&self) -> usize {
        usize::from(self.top) + usize::from(self.bottom)
    }

    fn corner(&self, symbol: &'static str, width: usize, style: RStyle) -> Row {
        pattern_fill(or_space(symbol), width, style)
    }

    fn edge_row(
        &self,
        inner_width: usize,
        edge: &'static str,
        start: &'static str,
        end: &'static str,
        style: RStyle,
    ) -> Row {
        let mut row = Vec::new();
        if self.left {
            row.extend(self.corner(start, self.left_width(), style));
        }
        row.extend(pattern_fill(or_space(edge), inner_width, style));
        if self.right {
            row.extend(self.corner(end, self.right_width(), style));
        }
        row
    }

    pub fn top_row(&self, inner_width: usize) -> Row {
        self.edge_row(
            inner_width,
            self.border.top,
            self.border.top_left,
            self.border.top_right,
            self.top_style,
        )
    }

    pub fn bottom_row(&self, inner_width: usize) -> Row {
        self.edge_row(
            inner_width,
            self.border.bottom,
            self.border.bottom_left,
            self.border.bottom_right,
            self.bottom_style,
        )
    }

    pub fn left_cells(&self, line: usize) -> Row {
        pattern_fill(
            nth_grapheme(self.border.left, line),
            self.left_width(),
            self.left_style,
        )
    }

    pub fn right_cells(&self, line: usize) -> Row {
        pattern_fill(
            nth_grapheme(self.border.right, line),
            self.right_width(),
            self.right_style,
        )
    }

    /// Wraps a block of equal-width rows with this border.
    pub fn apply(&self, rows: Vec<Row>) -> Vec<Row> {
        let inner_width = rows
            .iter()
            .map(|r| crate::cells::row_width(r))
            .max()
            .unwrap_or(0);
        let mut out = Vec::with_capacity(rows.len() + 2);
        if self.top {
            out.push(self.top_row(inner_width));
        }
        for (i, row) in rows.into_iter().enumerate() {
            let mut r: Vec<Cell> = Vec::new();
            if self.left {
                r.extend(self.left_cells(i));
            }
            r.extend(row);
            if self.right {
                r.extend(self.right_cells(i));
            }
            out.push(r);
        }
        if self.bottom {
            out.push(self.bottom_row(inner_width));
        }
        out
    }
}

/// Which border sides are enabled, convertible from CSS-like shorthands
/// (`true`, `(true, false)`, `(true, false, true)`, `(t, r, b, l)`).
pub type BorderSides = crate::Sides<bool>;
