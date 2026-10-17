//! Internal cell-grid representation used by the renderer.

use ratatui::style::Style as RStyle;
use ratatui::text::{Line, Span, Text};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::Position;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Cell {
    pub symbol: String,
    pub width: usize,
    pub style: RStyle,
}

impl Cell {
    pub fn new(symbol: &str, style: RStyle) -> Self {
        Self {
            symbol: symbol.to_string(),
            width: symbol.width(),
            style,
        }
    }

    pub fn blank(style: RStyle) -> Self {
        Self::new(" ", style)
    }

    fn is_space(&self) -> bool {
        self.symbol == " "
    }
}

pub(crate) type Row = Vec<Cell>;

pub(crate) fn row_width(row: &[Cell]) -> usize {
    row.iter().map(|c| c.width).sum()
}

pub(crate) fn block_width(rows: &[Row]) -> usize {
    rows.iter().map(|r| row_width(r)).max().unwrap_or(0)
}

/// Fills exactly `width` columns by cycling through the graphemes of
/// `pattern`. Graphemes that would overflow are replaced by spaces.
pub(crate) fn pattern_fill(pattern: &str, width: usize, style: RStyle) -> Row {
    let graphemes: Vec<&str> = pattern.graphemes(true).filter(|g| g.width() > 0).collect();
    let mut row = Vec::new();
    let mut w = 0;
    if graphemes.is_empty() {
        return (0..width).map(|_| Cell::blank(style)).collect();
    }
    let mut i = 0;
    while w < width {
        let g = graphemes[i % graphemes.len()];
        let gw = g.width();
        if w + gw > width {
            row.push(Cell::blank(style));
            w += 1;
        } else {
            row.push(Cell::new(g, style));
            w += gw;
        }
        i += 1;
    }
    row
}

/// Converts text into rows of cells, patching each span's style on top of
/// `base` (so styles already present on the content win).
pub(crate) fn rows_from_text(text: &Text<'_>, base: RStyle, tab_width: u16) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    let tab = " ".repeat(usize::from(tab_width));
    for line in &text.lines {
        let line_style = base.patch(text.style).patch(line.style);
        rows.push(Vec::new());
        for span in &line.spans {
            let style = line_style.patch(span.style);
            for g in span.content.graphemes(true) {
                match g {
                    "\n" | "\r\n" => rows.push(Vec::new()),
                    "\t" => {
                        let row = rows.last_mut().expect("row exists");
                        row.extend(tab.chars().map(|_| Cell::blank(style)));
                    }
                    g if g.chars().all(char::is_control) => {}
                    g if g.width() == 0 => {
                        if let Some(last) = rows.last_mut().and_then(|r| r.last_mut()) {
                            last.symbol.push_str(g);
                        }
                    }
                    g => rows
                        .last_mut()
                        .expect("row exists")
                        .push(Cell::new(g, style)),
                }
            }
        }
    }
    if rows.is_empty() {
        rows.push(Vec::new());
    }
    rows
}

pub(crate) fn rows_to_text(rows: Vec<Row>) -> Text<'static> {
    let lines: Vec<Line<'static>> = rows
        .into_iter()
        .map(|row| {
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut current = String::new();
            let mut current_style: Option<RStyle> = None;
            for cell in row {
                if current_style == Some(cell.style) {
                    current.push_str(&cell.symbol);
                } else {
                    if let Some(style) = current_style {
                        spans.push(Span::styled(std::mem::take(&mut current), style));
                    }
                    current_style = Some(cell.style);
                    current.push_str(&cell.symbol);
                }
            }
            if let Some(style) = current_style {
                spans.push(Span::styled(current, style));
            }
            Line::from(spans)
        })
        .collect();
    Text::from(lines)
}

/// Greedy word wrap of a single row to `width` columns.
pub(crate) fn wrap_row(row: Row, width: usize) -> Vec<Row> {
    if width == 0 || row_width(&row) <= width {
        return vec![row];
    }
    // Tokenize into runs of spaces / non-spaces.
    let mut tokens: Vec<(bool, Row)> = Vec::new();
    for cell in row {
        let space = cell.is_space();
        match tokens.last_mut() {
            Some((s, tok)) if *s == space => tok.push(cell),
            _ => tokens.push((space, vec![cell])),
        }
    }

    let mut out: Vec<Row> = Vec::new();
    let mut line: Row = Vec::new();
    let mut line_w = 0;
    let finish = |line: &mut Row, out: &mut Vec<Row>| {
        while line.last().is_some_and(Cell::is_space) {
            line.pop();
        }
        out.push(std::mem::take(line));
    };

    for (is_space, tok) in tokens {
        let tw = row_width(&tok);
        if is_space {
            if line.is_empty() && !out.is_empty() {
                continue; // drop leading spaces on wrapped lines
            }
            if line_w + tw <= width {
                line.extend(tok);
                line_w += tw;
            } else {
                finish(&mut line, &mut out);
                line_w = 0;
            }
            continue;
        }
        if line_w + tw <= width {
            line.extend(tok);
            line_w += tw;
        } else if tw <= width {
            finish(&mut line, &mut out);
            line = tok;
            line_w = tw;
        } else {
            for cell in tok {
                if line_w + cell.width > width && line_w > 0 {
                    finish(&mut line, &mut out);
                    line_w = 0;
                }
                line_w += cell.width;
                line.push(cell);
            }
        }
    }
    if !line.is_empty() || out.is_empty() {
        finish(&mut line, &mut out);
    }
    out
}

/// Truncates a row to at most `width` columns.
pub(crate) fn truncate_row(row: &mut Row, width: usize) {
    let mut w = 0;
    let mut keep = 0;
    for cell in row.iter() {
        if w + cell.width > width {
            break;
        }
        w += cell.width;
        keep += 1;
    }
    row.truncate(keep);
}

/// Pads a row to `width` columns, placing it at `pos`.
pub(crate) fn align_row(row: Row, width: usize, pos: Position, fill: &str, style: RStyle) -> Row {
    let w = row_width(&row);
    if w >= width {
        return row;
    }
    let (left, right) = pos.split(width - w);
    let mut out = pattern_fill(fill, left, style);
    out.extend(row);
    out.extend(pattern_fill(fill, right, style));
    out
}

/// Pads a block of rows vertically to `height` rows, placing it at `pos`.
pub(crate) fn align_rows_vertical(
    rows: Vec<Row>,
    height: usize,
    pos: Position,
    width: usize,
    fill: &str,
    style: RStyle,
) -> Vec<Row> {
    if rows.len() >= height {
        return rows;
    }
    let (top, bottom) = pos.split(height - rows.len());
    let mut out: Vec<Row> = (0..top).map(|_| pattern_fill(fill, width, style)).collect();
    out.extend(rows);
    out.extend((0..bottom).map(|_| pattern_fill(fill, width, style)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| r.iter().map(|c| c.symbol.as_str()).collect())
            .collect()
    }

    fn row(s: &str) -> Row {
        rows_from_text(&Text::from(s), RStyle::default(), 4).remove(0)
    }

    #[test]
    fn wraps_words() {
        let rows = wrap_row(row("the quick brown fox"), 10);
        assert_eq!(plain(&rows), ["the quick", "brown fox"]);
    }

    #[test]
    fn wraps_long_words() {
        let rows = wrap_row(row("abcdefghij kl"), 4);
        assert_eq!(plain(&rows), ["abcd", "efgh", "ij", "kl"]);
    }

    #[test]
    fn wide_chars() {
        let r = row("你好世界");
        assert_eq!(row_width(&r), 8);
        let rows = wrap_row(r, 5);
        assert_eq!(plain(&rows), ["你好", "世界"]);
        let mut r = row("你好");
        truncate_row(&mut r, 3);
        assert_eq!(plain(&[r]), ["你"]);
    }

    #[test]
    fn tabs_and_controls() {
        let rows = rows_from_text(&Text::from("a\tb\r"), RStyle::default(), 2);
        assert_eq!(plain(&rows), ["a  b"]);
        let rows = rows_from_text(&Text::from("a\tb"), RStyle::default(), 0);
        assert_eq!(plain(&rows), ["ab"]);
    }

    #[test]
    fn pattern_fill_cycles() {
        let r = pattern_fill("ab", 5, RStyle::default());
        assert_eq!(plain(&[r]), ["ababa"]);
        let r = pattern_fill("你", 3, RStyle::default());
        assert_eq!(plain(&[r]), ["你 "]);
    }
}
