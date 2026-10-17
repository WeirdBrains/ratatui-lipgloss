//! Wrapping ratatui widgets with a [`Style`].

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style as RStyle;
use ratatui::widgets::{StatefulWidget, Widget};

use crate::cells::Row;
use crate::color::has_dark_background;
use crate::{Sides, Style};

/// The rectangles produced by laying out a [`Style`]'s box model inside an
/// area. See [`Style::layout`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BoxLayout {
    /// The full area (after `max_width` / `max_height` clamping).
    pub area: Rect,
    /// The area inside the margins.
    pub outer: Rect,
    /// The bordered box (border + padding + content), positioned inside
    /// `outer` according to the alignment when `width` / `height` are set.
    pub border: Rect,
    /// The area inside the border (padding + content).
    pub padding: Rect,
    /// The area left for the content.
    pub content: Rect,
}

fn shrink(r: Rect, s: Sides) -> Rect {
    let x = r.x.saturating_add(s.left).min(r.right());
    let y = r.y.saturating_add(s.top).min(r.bottom());
    let width = r.width.saturating_sub(s.left.saturating_add(s.right));
    let height = r.height.saturating_sub(s.top.saturating_add(s.bottom));
    Rect::new(x, y, width, height)
}

impl Style {
    /// Computes the box model of this style inside `area`.
    ///
    /// * `max_width` / `max_height` clamp the area.
    /// * Margins are taken from the outside.
    /// * If `width` / `height` are set, the box (padding + content, plus
    ///   borders) gets that size and is placed inside the remaining area
    ///   using the horizontal / vertical alignment; otherwise it fills it.
    /// * Borders and padding are taken from the box; the rest is content.
    ///
    /// ```
    /// use ratatui::layout::Rect;
    /// use ratatui_lipgloss::{Border, Style};
    ///
    /// let s = Style::new().margin(1).border(Border::normal()).padding((0, 1));
    /// let l = s.layout(Rect::new(0, 0, 20, 10));
    /// assert_eq!(l.content, Rect::new(3, 2, 14, 6));
    /// ```
    #[must_use]
    pub fn layout(&self, area: Rect) -> BoxLayout {
        let mut area = area;
        if let Some(mw) = self.max_width {
            area.width = area.width.min(mw);
        }
        if let Some(mh) = self.max_height {
            area.height = area.height.min(mh);
        }
        let outer = shrink(area, self.get_margin());
        let spec = self.border_spec(true);
        let bl = u16::try_from(spec.left_width()).unwrap_or(u16::MAX);
        let br = u16::try_from(spec.right_width()).unwrap_or(u16::MAX);
        let bt = u16::from(spec.top);
        let bb = u16::from(spec.bottom);

        let box_w = self
            .width
            .map_or(outer.width, |w| w.saturating_add(bl + br).min(outer.width));
        let box_h = self.height.map_or(outer.height, |h| {
            h.saturating_add(bt + bb).min(outer.height)
        });
        let (dx, _) = self
            .get_align_horizontal()
            .split(usize::from(outer.width - box_w));
        let (dy, _) = self
            .get_align_vertical()
            .split(usize::from(outer.height - box_h));
        let border = Rect::new(
            outer.x + u16::try_from(dx).unwrap_or(0),
            outer.y + u16::try_from(dy).unwrap_or(0),
            box_w,
            box_h,
        );
        let padding = shrink(border, Sides::new(bt, br, bb, bl));
        let content = shrink(padding, self.get_padding());
        BoxLayout {
            area,
            outer,
            border,
            padding,
            content,
        }
    }

    /// The area available to content inside `area` (see [`Style::layout`]).
    #[must_use]
    pub fn inner(&self, area: Rect) -> Rect {
        self.layout(area).content
    }

    /// Wraps a widget so that it is rendered inside this style's margin,
    /// border and padding, with this style's inline rules applied to the
    /// whole box.
    ///
    /// Works with both [`Widget`]s and [`StatefulWidget`]s.
    #[must_use]
    pub fn wrap<W>(&self, widget: W) -> StyledWidget<W> {
        StyledWidget {
            style: self.clone(),
            widget,
        }
    }

    /// Draws the frame (margins, padding background, border) of this style
    /// into `buf` and returns the content area. This is what
    /// [`StyledWidget`] does before rendering its inner widget; use it
    /// directly for custom rendering.
    pub fn render_frame(&self, area: Rect, buf: &mut Buffer) -> Rect {
        let dark = has_dark_background();
        let l = self.layout(area.intersection(buf.area));

        // Margins: only painted if they have a color or a fill character.
        if self.margin_background.is_some() || self.margin_char.is_some() {
            let ms = RStyle {
                bg: self.margin_background.and_then(|c| c.resolve(dark)),
                ..RStyle::default()
            };
            let symbol = self.margin_char.unwrap_or(' ').to_string();
            fill_except(buf, l.area, l.outer, &symbol, ms);
        }

        // Padding + content get the inline style; padding cells are cleared.
        buf.set_style(l.padding, self.to_ratatui_with(dark));
        let pad_symbol = self.padding_char.unwrap_or(' ').to_string();
        let ws = self.whitespace_style(dark);
        fill_except(buf, l.padding, l.content, &pad_symbol, ws);

        // Border.
        let spec = self.border_spec(dark);
        let b = l.border;
        if b.width > 0 && b.height > 0 {
            let inner_w = usize::from(l.padding.width);
            if spec.top {
                write_row(buf, b.x, b.y, &spec.top_row(inner_w), b.width);
            }
            if spec.bottom && (b.height > 1 || !spec.top) {
                write_row(buf, b.x, b.bottom() - 1, &spec.bottom_row(inner_w), b.width);
            }
            for (i, y) in (l.padding.y..l.padding.bottom()).enumerate() {
                if spec.left {
                    write_row(buf, b.x, y, &spec.left_cells(i), b.width);
                }
                if spec.right {
                    let rw = u16::try_from(spec.right_width()).unwrap_or(0);
                    if b.width >= rw {
                        write_row(buf, b.right() - rw, y, &spec.right_cells(i), rw);
                    }
                }
            }
        }
        l.content
    }
}

fn fill_except(buf: &mut Buffer, outer: Rect, inner: Rect, symbol: &str, style: RStyle) {
    for y in outer.top()..outer.bottom() {
        for x in outer.left()..outer.right() {
            let inside =
                x >= inner.left() && x < inner.right() && y >= inner.top() && y < inner.bottom();
            if !inside && let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(symbol).set_style(style);
            }
        }
    }
}

fn write_row(buf: &mut Buffer, x: u16, y: u16, row: &Row, max_width: u16) {
    let mut cx = x;
    let end = x.saturating_add(max_width);
    for cell in row {
        if cx >= end {
            break;
        }
        let (nx, _) = buf.set_stringn(cx, y, &cell.symbol, usize::from(end - cx), cell.style);
        cx = if nx == cx {
            cx.saturating_add(u16::try_from(cell.width.max(1)).unwrap_or(1))
        } else {
            nx
        };
    }
}

/// A widget rendered inside a [`Style`]'s box (margin, border, padding,
/// sizing and alignment). Create it with [`Style::wrap`] or
/// [`WidgetStyleExt::styled`].
///
/// ```
/// use ratatui::{buffer::Buffer, layout::Rect, widgets::{Paragraph, Widget}};
/// use ratatui_lipgloss::{Border, Style, WidgetStyleExt};
///
/// let style = Style::new().border(Border::rounded()).padding((0, 1));
/// let mut buf = Buffer::empty(Rect::new(0, 0, 8, 3));
/// Paragraph::new("hi").styled(&style).render(buf.area, &mut buf);
/// assert_eq!(buf, Buffer::with_lines(["╭──────╮", "│ hi   │", "╰──────╯"]));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyledWidget<W> {
    style: Style,
    widget: W,
}

impl<W> StyledWidget<W> {
    /// Creates a styled widget.
    #[must_use]
    pub fn new(widget: W, style: Style) -> Self {
        Self { style, widget }
    }

    /// The style.
    #[must_use]
    pub const fn style(&self) -> &Style {
        &self.style
    }

    /// The wrapped widget.
    #[must_use]
    pub const fn widget(&self) -> &W {
        &self.widget
    }

    /// Unwraps the inner widget.
    #[must_use]
    pub fn into_inner(self) -> W {
        self.widget
    }

    /// The area the inner widget will be rendered into.
    #[must_use]
    pub fn inner(&self, area: Rect) -> Rect {
        self.style.inner(area)
    }
}

impl<W: Widget> Widget for StyledWidget<W> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let content = self.style.render_frame(area, buf);
        self.widget.render(content, buf);
    }
}

impl<W> Widget for &StyledWidget<W>
where
    for<'a> &'a W: Widget,
{
    fn render(self, area: Rect, buf: &mut Buffer) {
        let content = self.style.render_frame(area, buf);
        (&self.widget).render(content, buf);
    }
}

impl<W: StatefulWidget> StatefulWidget for StyledWidget<W> {
    type State = W::State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let content = self.style.render_frame(area, buf);
        StatefulWidget::render(self.widget, content, buf, state);
    }
}

/// Extension trait adding [`styled`](WidgetStyleExt::styled) to every ratatui
/// [`Widget`].
pub trait WidgetStyleExt: Sized {
    /// Wraps the widget in a [`StyledWidget`] using `style`.
    #[must_use]
    fn styled(self, style: &Style) -> StyledWidget<Self> {
        style.wrap(self)
    }
}

impl<W: Widget> WidgetStyleExt for W {}
