//! Tables, modeled on Lip Gloss's `table` package.
//!
//! ```
//! use ratatui_lipgloss::table::{Table, TableRow};
//! use ratatui_lipgloss::{Border, Position, Style};
//!
//! let t = Table::new()
//!     .border(Border::rounded())
//!     .headers(["LANGUAGE", "FORMAL"])
//!     .row(["Chinese", "Nǐn hǎo"])
//!     .row(["Spanish", "Hola"])
//!     .style_func(|row, _col| match row {
//!         TableRow::Header => Style::new().bold(true).padding((0, 1)).align(Position::CENTER),
//!         TableRow::Data(_) => Style::new().padding((0, 1)),
//!     });
//! assert_eq!(
//!     t.to_string(),
//!     "╭──────────┬─────────╮\n\
//!      │ LANGUAGE │ FORMAL  │\n\
//!      ├──────────┼─────────┤\n\
//!      │ Chinese  │ Nǐn hǎo │\n\
//!      │ Spanish  │ Hola    │\n\
//!      ╰──────────┴─────────╯"
//! );
//! ```
//!
//! `&Table` is a ratatui [`Widget`]; when it has no explicit
//! [`height`](Table::height) it uses the area's height, showing an overflow
//! row (`…`) if rows don't fit. Scroll with [`Table::y_offset`].

use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style as RStyle;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Widget;

use crate::cells::{Cell, pattern_fill, rows_from_text, rows_to_text, truncate_row};
use crate::{Border, DEFAULT_TAB_WIDTH, Position, Style, join_horizontal, to_ansi_string, width};

/// Which row a cell belongs to, passed to [`Table::style_func`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TableRow {
    /// The header row (Lip Gloss's `HeaderRow`).
    Header,
    /// A data row, by index into the table's rows.
    Data(usize),
}

/// Computes the style of the cell at `(row, column)`.
pub type StyleFunc = Arc<dyn Fn(TableRow, usize) -> Style + Send + Sync>;

/// A table builder (Lip Gloss's `table.Table`).
///
/// By default it has a [`Border::normal`] border on the outside, between
/// columns and below the header, but not between rows.
#[derive(Clone)]
pub struct Table {
    headers: Vec<Text<'static>>,
    rows: Vec<Vec<Text<'static>>>,
    style_func: Option<StyleFunc>,
    base_style: Style,
    border: Border,
    border_top: bool,
    border_bottom: bool,
    border_left: bool,
    border_right: bool,
    border_header: bool,
    border_column: bool,
    border_row: bool,
    border_style: Style,
    width: Option<u16>,
    height: Option<u16>,
    y_offset: usize,
    wrap: bool,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            headers: Vec::new(),
            rows: Vec::new(),
            style_func: None,
            base_style: Style::new(),
            border: Border::normal(),
            border_top: true,
            border_bottom: true,
            border_left: true,
            border_right: true,
            border_header: true,
            border_column: true,
            border_row: false,
            border_style: Style::new(),
            width: None,
            height: None,
            y_offset: 0,
            wrap: true,
        }
    }
}

impl fmt::Debug for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Table")
            .field("headers", &self.headers)
            .field("rows", &self.rows)
            .field("border", &self.border)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("y_offset", &self.y_offset)
            .finish_non_exhaustive()
    }
}

macro_rules! toggles {
    ($($(#[$doc:meta])* $field:ident, $get:ident;)*) => {$(
        $(#[$doc])*
        #[must_use]
        pub fn $field(mut self, v: bool) -> Self {
            self.$field = v;
            self
        }
        #[doc = concat!("Returns the `", stringify!($field), "` setting.")]
        #[must_use]
        pub fn $get(&self) -> bool {
            self.$field
        }
    )*};
}

fn texts<I, T>(cells: I) -> Vec<Text<'static>>
where
    I: IntoIterator<Item = T>,
    T: Into<Text<'static>>,
{
    cells.into_iter().map(Into::into).collect()
}

/// Truncates every line of `text` to `w` columns, ending with `…`.
fn truncate(text: &Text<'static>, w: usize) -> Text<'static> {
    if width(text) <= w {
        return text.clone();
    }
    let rows = rows_from_text(text, RStyle::default(), DEFAULT_TAB_WIDTH)
        .into_iter()
        .map(|mut row| {
            if crate::cells::row_width(&row) > w {
                truncate_row(&mut row, w.saturating_sub(1));
                let style = row.last().map(|c| c.style).unwrap_or_default();
                if w > 0 {
                    row.push(Cell::new("…", style));
                }
            }
            row
        })
        .collect();
    rows_to_text(rows)
}

fn fill(pattern: &'static str, n: usize, style: RStyle) -> Span<'static> {
    let p = if pattern.is_empty() { " " } else { pattern };
    let s: String = pattern_fill(p, n, style)
        .into_iter()
        .map(|c| c.symbol)
        .collect();
    Span::styled(s, style)
}

/// The resolved layout of a table.
struct Layout {
    widths: Vec<usize>,
    visible: Range<usize>,
    overflow: bool,
    height: Option<usize>,
}

impl Table {
    /// Creates an empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the header cells.
    #[must_use]
    pub fn headers<I, T>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Text<'static>>,
    {
        self.headers = texts(headers);
        self
    }

    /// Appends a row.
    #[must_use]
    pub fn row<I, T>(mut self, cells: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Text<'static>>,
    {
        self.rows.push(texts(cells));
        self
    }

    /// Appends several rows.
    #[must_use]
    pub fn rows<R, I, T>(mut self, rows: R) -> Self
    where
        R: IntoIterator<Item = I>,
        I: IntoIterator<Item = T>,
        T: Into<Text<'static>>,
    {
        self.rows.extend(rows.into_iter().map(texts));
        self
    }

    /// Removes all rows (headers are kept).
    #[must_use]
    pub fn clear_rows(mut self) -> Self {
        self.rows.clear();
        self
    }

    /// Sets the function computing each cell's style. The result inherits
    /// from the [`base_style`](Table::base_style).
    #[must_use]
    pub fn style_func<F>(mut self, f: F) -> Self
    where
        F: Fn(TableRow, usize) -> Style + Send + Sync + 'static,
    {
        self.style_func = Some(Arc::new(f));
        self
    }

    /// Sets the style every cell inherits from.
    #[must_use]
    pub fn base_style(mut self, style: Style) -> Self {
        self.base_style = style;
        self
    }

    /// Sets the border characters (the `middle_*` characters are used for
    /// separators).
    #[must_use]
    pub fn border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }

    /// Sets the style (colors) of the border.
    #[must_use]
    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    toggles! {
        /// Draws the top border.
        border_top, get_border_top;
        /// Draws the bottom border.
        border_bottom, get_border_bottom;
        /// Draws the left border.
        border_left, get_border_left;
        /// Draws the right border.
        border_right, get_border_right;
        /// Draws the separator below the header.
        border_header, get_border_header;
        /// Draws separators between columns.
        border_column, get_border_column;
        /// Draws separators between rows.
        border_row, get_border_row;
        /// Wraps long cell content (default `true`); otherwise it is
        /// truncated with `…`. Headers are always truncated.
        wrap, get_wrap;
    }

    /// Sets the total width (including borders). Columns grow or shrink
    /// (widest first) to fit.
    #[must_use]
    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    /// Sets the maximum total height (including borders). Rows that don't
    /// fit are replaced by an overflow row of `…`.
    #[must_use]
    pub fn height(mut self, height: u16) -> Self {
        self.height = Some(height);
        self
    }

    /// Sets the index of the first data row shown (scrolling).
    #[must_use]
    pub fn y_offset(mut self, offset: usize) -> Self {
        self.y_offset = offset;
        self
    }

    /// The header cells.
    #[must_use]
    pub fn get_headers(&self) -> &[Text<'static>] {
        &self.headers
    }

    /// The data rows.
    #[must_use]
    pub fn get_rows(&self) -> &[Vec<Text<'static>>] {
        &self.rows
    }

    /// The scroll offset.
    #[must_use]
    pub fn get_y_offset(&self) -> usize {
        self.y_offset
    }

    /// The configured height, if any.
    #[must_use]
    pub fn get_height(&self) -> Option<u16> {
        self.height
    }

    /// The configured width, if any.
    #[must_use]
    pub fn get_width(&self) -> Option<u16> {
        self.width
    }

    /// The range of data rows that are drawn, given the height and offset.
    #[must_use]
    pub fn visible_rows(&self) -> Range<usize> {
        self.layout().visible
    }

    fn columns(&self) -> usize {
        self.rows
            .iter()
            .map(Vec::len)
            .chain([self.headers.len()])
            .max()
            .unwrap_or(0)
    }

    fn style(&self, row: TableRow, col: usize) -> Style {
        let s = self
            .style_func
            .as_ref()
            .map_or_else(Style::new, |f| f(row, col));
        s.inherit(&self.base_style)
    }

    fn content(&self, row: TableRow, col: usize) -> Text<'static> {
        let cell = match row {
            TableRow::Header => self.headers.get(col),
            TableRow::Data(r) => self.rows.get(r).and_then(|r| r.get(col)),
        };
        cell.cloned().unwrap_or_default()
    }

    fn render_cell(&self, row: TableRow, col: usize, w: usize, h: Option<usize>) -> Text<'static> {
        let style = self.style(row, col);
        let p = style.get_padding();
        let frame_w = usize::from(style.get_horizontal_frame_size())
            .saturating_sub(usize::from(p.left + p.right));
        let frame_h = usize::from(style.get_vertical_frame_size())
            .saturating_sub(usize::from(p.top + p.bottom));
        let mut content = self.content(row, col);
        if !self.wrap || row == TableRow::Header {
            let inner = w.saturating_sub(frame_w + usize::from(p.left + p.right));
            content = truncate(&content, inner);
        }
        let mut s = style
            .width(u16::try_from(w.saturating_sub(frame_w)).unwrap_or(u16::MAX))
            .max_width(u16::try_from(w).unwrap_or(u16::MAX));
        if let Some(h) = h {
            s = s.height(u16::try_from(h.saturating_sub(frame_h)).unwrap_or(u16::MAX));
        }
        s.render(content)
    }

    fn row_kinds(&self) -> impl Iterator<Item = TableRow> + '_ {
        let header = (!self.headers.is_empty()).then_some(TableRow::Header);
        header
            .into_iter()
            .chain((0..self.rows.len()).map(TableRow::Data))
    }

    fn layout(&self) -> Layout {
        let cols = self.columns();
        let mut widths = vec![0usize; cols];
        for kind in self.row_kinds() {
            for (c, w) in widths.iter_mut().enumerate() {
                let style = self.style(kind, c);
                *w = (*w).max(width(&style.render(self.content(kind, c))));
            }
        }
        for w in &mut widths {
            *w = (*w).max(1);
        }
        if let Some(total) = self.width {
            let frame = usize::from(self.border_left)
                + usize::from(self.border_right)
                + if self.border_column {
                    cols.saturating_sub(1)
                } else {
                    0
                };
            let target = usize::from(total).saturating_sub(frame).max(cols);
            let mut sum: usize = widths.iter().sum();
            // Grow the narrowest column first (evens columns out), shrink the
            // widest first.
            while sum < target && cols > 0 {
                let (idx, _) = widths
                    .iter()
                    .copied()
                    .enumerate()
                    .min_by_key(|&(i, w)| (w, i))
                    .unwrap_or((0, 0));
                widths[idx] += 1;
                sum += 1;
            }
            while sum > target {
                let (idx, max) = widths
                    .iter()
                    .copied()
                    .enumerate()
                    .max_by_key(|&(i, w)| (w, usize::MAX - i))
                    .unwrap_or((0, 1));
                if max <= 1 {
                    break;
                }
                widths[idx] -= 1;
                sum -= 1;
            }
        }

        let start = self.y_offset.min(self.rows.len());
        let Some(height) = self.height.map(usize::from) else {
            return Layout {
                widths,
                visible: start..self.rows.len(),
                overflow: false,
                height: None,
            };
        };
        let row_h = |kind: TableRow| {
            (0..cols)
                .map(|c| self.render_cell(kind, c, widths[c], None).lines.len())
                .max()
                .unwrap_or(1)
        };
        let mut used = usize::from(self.border_top) + usize::from(self.border_bottom);
        if !self.headers.is_empty() {
            used += row_h(TableRow::Header) + usize::from(self.border_header);
        }
        let sep = usize::from(self.border_row);
        let mut taken: Vec<usize> = Vec::new();
        for r in start..self.rows.len() {
            let need = row_h(TableRow::Data(r)) + if taken.is_empty() { 0 } else { sep };
            if used + need > height {
                break;
            }
            used += need;
            taken.push(need);
        }
        let end = start + taken.len();
        let overflow = end < self.rows.len();
        if overflow {
            // Make room for the overflow row (and its separator).
            let extra = |taken: &[usize]| if taken.is_empty() { 1 } else { 1 + sep };
            while used + extra(&taken) > height {
                match taken.pop() {
                    Some(n) => used -= n,
                    None => break,
                }
            }
        }
        Layout {
            widths,
            visible: start..start + taken.len(),
            overflow,
            height: Some(height),
        }
    }

    fn border_line(
        &self,
        widths: &[usize],
        (left, fill_char, middle, right): (&'static str, &'static str, &'static str, &'static str),
    ) -> Line<'static> {
        let style = self.border_style.to_ratatui();
        let mut spans = Vec::new();
        if self.border_left {
            spans.push(fill(left, 1, style));
        }
        for (i, w) in widths.iter().enumerate() {
            spans.push(fill(fill_char, *w, style));
            if self.border_column && i + 1 < widths.len() {
                spans.push(fill(middle, 1, style));
            }
        }
        if self.border_right {
            spans.push(fill(right, 1, style));
        }
        Line::from(spans)
    }

    fn content_row(&self, kind: TableRow, widths: &[usize], overflow: bool) -> Vec<Line<'static>> {
        let cells: Vec<Text<'static>> = if overflow {
            (0..widths.len())
                .map(|c| {
                    let style = self.style(kind, c);
                    let p = style.get_padding();
                    let frame_w = usize::from(style.get_horizontal_frame_size())
                        .saturating_sub(usize::from(p.left + p.right));
                    style
                        .padding_top(0)
                        .padding_bottom(0)
                        .width(u16::try_from(widths[c].saturating_sub(frame_w)).unwrap_or(0))
                        .max_width(u16::try_from(widths[c]).unwrap_or(0))
                        .render("…")
                })
                .collect()
        } else {
            let h = (0..widths.len())
                .map(|c| self.render_cell(kind, c, widths[c], None).lines.len())
                .max()
                .unwrap_or(1);
            (0..widths.len())
                .map(|c| self.render_cell(kind, c, widths[c], Some(h)))
                .collect()
        };
        let h = cells.iter().map(|t| t.lines.len()).max().unwrap_or(1);
        let style = self.border_style.to_ratatui();
        let column = |s: &'static str| {
            Text::from(
                (0..h)
                    .map(|_| Line::from(fill(s, 1, style)))
                    .collect::<Vec<_>>(),
            )
        };
        let mut parts = Vec::new();
        if self.border_left {
            parts.push(column(self.border.left));
        }
        let n = cells.len();
        for (i, (cell, w)) in cells.into_iter().zip(widths).enumerate() {
            parts.push(crate::place(
                u16::try_from(*w).unwrap_or(u16::MAX),
                u16::try_from(h).unwrap_or(u16::MAX),
                Position::LEFT,
                Position::TOP,
                cell,
            ));
            if self.border_column && i + 1 < n {
                parts.push(column(self.border.left));
            }
        }
        if self.border_right {
            parts.push(column(self.border.right));
        }
        join_horizontal(Position::TOP, parts).lines
    }

    /// Renders the table to a ratatui [`Text`].
    #[must_use]
    pub fn render(&self) -> Text<'static> {
        if self.columns() == 0 {
            return Text::default();
        }
        let layout = self.layout();
        let w = &layout.widths;
        let b = self.border;
        let mut lines: Vec<Line<'static>> = Vec::new();
        if self.border_top {
            lines.push(self.border_line(w, (b.top_left, b.top, b.middle_top, b.top_right)));
        }
        if !self.headers.is_empty() {
            lines.extend(self.content_row(TableRow::Header, w, false));
            if self.border_header {
                lines.push(self.border_line(w, (b.middle_left, b.top, b.middle, b.middle_right)));
            }
        }
        for (k, r) in layout.visible.clone().enumerate() {
            if k > 0 && self.border_row {
                lines
                    .push(self.border_line(w, (b.middle_left, b.bottom, b.middle, b.middle_right)));
            }
            lines.extend(self.content_row(TableRow::Data(r), w, false));
        }
        if layout.overflow {
            if self.border_row && !layout.visible.is_empty() {
                lines
                    .push(self.border_line(w, (b.middle_left, b.bottom, b.middle, b.middle_right)));
            }
            lines.extend(self.content_row(TableRow::Data(layout.visible.end), w, true));
        }
        if self.border_bottom {
            lines.push(self.border_line(
                w,
                (b.bottom_left, b.bottom, b.middle_bottom, b.bottom_right),
            ));
        }
        if let Some(h) = layout.height {
            // Like Lip Gloss, a table taller than its height is clipped.
            lines.truncate(h);
        }
        Text::from(lines)
    }

    /// Renders the table to a string with ANSI escape codes.
    #[must_use]
    pub fn render_ansi(&self) -> String {
        to_ansi_string(&self.render())
    }
}

impl fmt::Display for Table {
    /// Writes the table as plain text (no styling).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&crate::to_plain_string(&self.render()))
    }
}

impl Widget for &Table {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = if self.height.is_none() {
            self.clone().height(area.height).render()
        } else {
            Table::render(self)
        };
        Widget::render(text, area, buf);
    }
}
