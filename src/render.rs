//! Rendering a [`Style`] to ratatui [`Text`] (and to ANSI strings).

use core::fmt;

use ratatui::style::Style as RStyle;
use ratatui::text::Text;

use crate::Style;
use crate::cells::{
    Cell, Row, align_row, align_rows_vertical, block_width, pattern_fill, row_width,
    rows_from_text, rows_to_text, truncate_row, wrap_row,
};
use crate::color::has_dark_background;

impl Style {
    /// Renders content with every rule of this style applied, returning an
    /// owned ratatui [`Text`].
    ///
    /// `content` can be anything that converts into `Text`: `&str`, `String`,
    /// `Span`, `Line` or `Text`. Styles already present on the content are
    /// preserved and take precedence over this style's inline rules.
    ///
    /// The result is a regular ratatui widget (`frame.render_widget(text,
    /// area)`), can be put in a `Paragraph`, measured with
    /// [`width`](crate::width) / [`height`](crate::height), or combined with
    /// [`join_horizontal`](crate::join_horizontal),
    /// [`join_vertical`](crate::join_vertical) and [`place`](crate::place).
    ///
    /// Layout order: tabs → wrap (to `width - horizontal padding`) →
    /// horizontal alignment (to `width`) → vertical alignment (to `height`) →
    /// padding → border → margin → `max_width` / `max_height` truncation.
    #[must_use]
    pub fn render<'a>(&self, content: impl Into<Text<'a>>) -> Text<'static> {
        self.render_with(content, has_dark_background())
    }

    /// Like [`Style::render`] but with an explicit light/dark background
    /// flag for adaptive colors.
    #[must_use]
    pub fn render_with<'a>(
        &self,
        content: impl Into<Text<'a>>,
        dark_background: bool,
    ) -> Text<'static> {
        rows_to_text(self.render_rows(&content.into(), dark_background))
    }

    /// Renders the string set with [`Style::set_string`] (empty if unset).
    #[must_use]
    pub fn render_value(&self) -> Text<'static> {
        self.render(Text::default())
    }

    /// Renders content and serializes it to a string with ANSI escape codes,
    /// ready to `println!` outside of a TUI (Lip Gloss's `Render` +
    /// `Println`).
    #[must_use]
    pub fn render_ansi<'a>(&self, content: impl Into<Text<'a>>) -> String {
        crate::to_ansi_string(&self.render(content))
    }

    /// Renders content and returns just the characters, without styling.
    #[must_use]
    pub fn render_plain<'a>(&self, content: impl Into<Text<'a>>) -> String {
        crate::to_plain_string(&self.render(content))
    }

    pub(crate) fn render_rows(&self, content: &Text<'_>, dark: bool) -> Vec<Row> {
        let text_style = self.to_ratatui_with(dark);
        let ws = self.whitespace_style(dark);
        let tab = self.get_tab_width();
        let pad_char = self.padding_char.unwrap_or(' ').to_string();
        let margin_char = self.margin_char.unwrap_or(' ').to_string();

        // 1. Content (with the optional underlying string value).
        let mut rows = match &self.value {
            Some(v) => {
                let mut rows = rows_from_text(&Text::from(v.as_str()), text_style, tab);
                let has_content = content.lines.iter().any(|l| l.width() > 0);
                if has_content {
                    let mut more = rows_from_text(content, text_style, tab).into_iter();
                    if let (Some(last), Some(first)) = (rows.last_mut(), more.next()) {
                        last.push(Cell::blank(text_style));
                        last.extend(first);
                    }
                    rows.extend(more);
                }
                rows
            }
            None => rows_from_text(content, text_style, tab),
        };

        let inline = self.get_inline();
        let pad = if inline {
            crate::Sides::all(0)
        } else {
            self.get_padding()
        };
        let (pl, pr, pt, pb) = (
            usize::from(pad.left),
            usize::from(pad.right),
            usize::from(pad.top),
            usize::from(pad.bottom),
        );

        if inline {
            rows = vec![rows.into_iter().flatten().collect()];
        }

        // 2. Word wrap.
        let width = self.width.map(usize::from);
        if let Some(w) = width {
            let wrap_at = w.saturating_sub(pl + pr);
            if !inline && wrap_at > 0 {
                rows = rows
                    .into_iter()
                    .flat_map(|r| wrap_row(r, wrap_at))
                    .collect();
            }
        }

        // 3. Horizontal alignment.
        let content_width = block_width(&rows).max(width.map_or(0, |w| w.saturating_sub(pl + pr)));
        let h = self.get_align_horizontal();
        rows = rows
            .into_iter()
            .map(|r| align_row(r, content_width, h, " ", ws))
            .collect();

        // 4. Vertical alignment / height.
        if let Some(height) = self.height.map(usize::from) {
            let content_height = height.saturating_sub(pt + pb);
            rows = align_rows_vertical(
                rows,
                content_height,
                self.get_align_vertical(),
                content_width,
                " ",
                ws,
            );
        }

        if !inline {
            // 5. Padding.
            if pl + pr > 0 {
                rows = rows
                    .into_iter()
                    .map(|r| {
                        let mut out = pattern_fill(&pad_char, pl, ws);
                        out.extend(r);
                        out.extend(pattern_fill(&pad_char, pr, ws));
                        out
                    })
                    .collect();
            }
            let full = content_width + pl + pr;
            if pt + pb > 0 {
                let mut out: Vec<Row> =
                    (0..pt).map(|_| pattern_fill(&pad_char, full, ws)).collect();
                out.append(&mut rows);
                out.extend((0..pb).map(|_| pattern_fill(&pad_char, full, ws)));
                rows = out;
            }

            // 6. Border.
            let spec = self.border_spec(dark);
            if spec.top || spec.right || spec.bottom || spec.left {
                rows = spec.apply(rows);
            }

            // 7. Margins.
            let m = self.get_margin();
            let mstyle = RStyle {
                bg: self.margin_background.and_then(|c| c.resolve(dark)),
                ..RStyle::default()
            };
            let (ml, mr) = (usize::from(m.left), usize::from(m.right));
            if ml + mr > 0 {
                rows = rows
                    .into_iter()
                    .map(|r| {
                        let mut out = pattern_fill(&margin_char, ml, mstyle);
                        out.extend(r);
                        out.extend(pattern_fill(&margin_char, mr, mstyle));
                        out
                    })
                    .collect();
            }
            let total = block_width(&rows);
            // Make every row the same width (margin rows included).
            rows = rows
                .into_iter()
                .map(|r| {
                    let w = row_width(&r);
                    let mut r = r;
                    if w < total {
                        r.extend(pattern_fill(&margin_char, total - w, mstyle));
                    }
                    r
                })
                .collect();
            if m.top + m.bottom > 0 {
                let mut out: Vec<Row> = (0..m.top)
                    .map(|_| pattern_fill(&margin_char, total, mstyle))
                    .collect();
                out.append(&mut rows);
                out.extend((0..m.bottom).map(|_| pattern_fill(&margin_char, total, mstyle)));
                rows = out;
            }
        }

        // 8. Max sizes.
        if let Some(mw) = self.max_width {
            for r in &mut rows {
                truncate_row(r, usize::from(mw));
            }
        }
        if let Some(mh) = self.max_height {
            rows.truncate(usize::from(mh));
        }
        rows
    }
}

impl fmt::Display for Style {
    /// Writes [`Style::render_value`] as an ANSI string (Lip Gloss's
    /// `Stringer`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&crate::to_ansi_string(&self.render_value()))
    }
}
