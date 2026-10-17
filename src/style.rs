//! The [`Style`] builder.

use ratatui::style::{Modifier, Style as RStyle, Styled};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};

use crate::border::{BorderSides, BorderSpec};
use crate::color::{TerminalColor, has_dark_background};
use crate::{Border, Position, Sides};

/// Tabs are converted to this many spaces unless [`Style::tab_width`] is set.
pub const DEFAULT_TAB_WIDTH: u16 = 4;

/// A declarative, chainable style definition, modeled on Lip Gloss's
/// `lipgloss.Style`.
///
/// A `Style` is a plain value: every builder method takes `self` and returns
/// a new style, so cloning is copying and nothing is ever mutated behind your
/// back. Every rule is optional: an *unset* rule is different from a rule
/// set to its default, which is what makes [`Style::inherit`] work.
///
/// A style can be used in three ways:
///
/// 1. **Inline**: convert it to a ratatui [`Style`](ratatui::style::Style)
///    (it implements `Into<ratatui::style::Style>`), so it works anywhere
///    ratatui accepts a style: `Span::styled("hi", &style)`,
///    `Paragraph::new(..).style(&style)`, or [`Style::apply`].
/// 2. **Rendered**: [`Style::render`] lays out content (wrapping, padding,
///    alignment, sizing, borders, margins) and returns a ratatui
///    [`Text`](ratatui::text::Text) that you can render as a widget or
///    compose further with [`join_horizontal`](crate::join_horizontal),
///    [`join_vertical`](crate::join_vertical) and [`place`](crate::place).
/// 3. **Around a widget**: [`Style::wrap`] (or
///    [`WidgetStyleExt::styled`](crate::WidgetStyleExt::styled)) wraps any
///    ratatui widget, drawing margin, border and padding around it.
///
/// ```
/// use ratatui_lipgloss::{Border, Style};
///
/// let style = Style::new()
///     .bold(true)
///     .foreground("#FAFAFA")
///     .background("#7D56F4")
///     .padding_top(2)
///     .padding_left(4)
///     .width(22);
///
/// let text = style.render("Hello, kitty");
/// assert_eq!(text.lines.len(), 3);
/// assert_eq!(text.lines[2].width(), 22);
///
/// let boxed = Style::new().border(Border::rounded()).padding((0, 1));
/// assert_eq!(
///     ratatui_lipgloss::to_plain_string(&boxed.render("hi")),
///     "╭────╮\n│ hi │\n╰────╯"
/// );
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub(crate) value: Option<String>,

    pub(crate) foreground: Option<TerminalColor>,
    pub(crate) background: Option<TerminalColor>,
    pub(crate) underline_color: Option<TerminalColor>,
    pub(crate) bold: Option<bool>,
    pub(crate) italic: Option<bool>,
    pub(crate) underline: Option<bool>,
    pub(crate) strikethrough: Option<bool>,
    pub(crate) faint: Option<bool>,
    pub(crate) reverse: Option<bool>,
    pub(crate) blink: Option<bool>,
    pub(crate) hidden: Option<bool>,

    pub(crate) width: Option<u16>,
    pub(crate) height: Option<u16>,
    pub(crate) max_width: Option<u16>,
    pub(crate) max_height: Option<u16>,
    pub(crate) align_horizontal: Option<Position>,
    pub(crate) align_vertical: Option<Position>,

    pub(crate) padding_top: Option<u16>,
    pub(crate) padding_right: Option<u16>,
    pub(crate) padding_bottom: Option<u16>,
    pub(crate) padding_left: Option<u16>,
    pub(crate) padding_char: Option<char>,

    pub(crate) margin_top: Option<u16>,
    pub(crate) margin_right: Option<u16>,
    pub(crate) margin_bottom: Option<u16>,
    pub(crate) margin_left: Option<u16>,
    pub(crate) margin_char: Option<char>,
    pub(crate) margin_background: Option<TerminalColor>,

    pub(crate) border_style: Option<Border>,
    pub(crate) border_top: Option<bool>,
    pub(crate) border_right: Option<bool>,
    pub(crate) border_bottom: Option<bool>,
    pub(crate) border_left: Option<bool>,
    pub(crate) border_top_foreground: Option<TerminalColor>,
    pub(crate) border_right_foreground: Option<TerminalColor>,
    pub(crate) border_bottom_foreground: Option<TerminalColor>,
    pub(crate) border_left_foreground: Option<TerminalColor>,
    pub(crate) border_top_background: Option<TerminalColor>,
    pub(crate) border_right_background: Option<TerminalColor>,
    pub(crate) border_bottom_background: Option<TerminalColor>,
    pub(crate) border_left_background: Option<TerminalColor>,

    pub(crate) inline: Option<bool>,
    pub(crate) tab_width: Option<u16>,
    pub(crate) color_whitespace: Option<bool>,
}

macro_rules! bool_rules {
    ($($(#[$doc:meta])* $field:ident, $unset:ident, $get:ident;)*) => {$(
        $(#[$doc])*
        #[must_use]
        pub fn $field(mut self, v: bool) -> Self {
            self.$field = Some(v);
            self
        }
        #[doc = concat!("Removes the `", stringify!($field), "` rule.")]
        #[must_use]
        pub fn $unset(mut self) -> Self {
            self.$field = None;
            self
        }
        #[doc = concat!("Returns the `", stringify!($field), "` rule (`false` if unset).")]
        #[must_use]
        pub fn $get(&self) -> bool {
            self.$field.unwrap_or(false)
        }
    )*};
}

macro_rules! color_rules {
    ($($(#[$doc:meta])* $field:ident, $unset:ident, $get:ident;)*) => {$(
        $(#[$doc])*
        #[must_use]
        pub fn $field(mut self, c: impl Into<TerminalColor>) -> Self {
            self.$field = Some(c.into());
            self
        }
        #[doc = concat!("Removes the `", stringify!($field), "` rule.")]
        #[must_use]
        pub fn $unset(mut self) -> Self {
            self.$field = None;
            self
        }
        #[doc = concat!("Returns the `", stringify!($field), "` rule, if set.")]
        #[must_use]
        pub fn $get(&self) -> Option<TerminalColor> {
            self.$field
        }
    )*};
}

macro_rules! u16_rules {
    ($($(#[$doc:meta])* $field:ident, $unset:ident, $get:ident;)*) => {$(
        $(#[$doc])*
        #[must_use]
        pub fn $field(mut self, v: u16) -> Self {
            self.$field = Some(v);
            self
        }
        #[doc = concat!("Removes the `", stringify!($field), "` rule.")]
        #[must_use]
        pub fn $unset(mut self) -> Self {
            self.$field = None;
            self
        }
        #[doc = concat!("Returns the `", stringify!($field), "` rule (`0` if unset).")]
        #[must_use]
        pub fn $get(&self) -> u16 {
            self.$field.unwrap_or(0)
        }
    )*};
}

impl Style {
    /// Creates an empty style with no rules set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets an underlying string value, rendered by [`Style::render_value`]
    /// and by [`Display`](core::fmt::Display). When set, [`Style::render`]
    /// prepends it to the content, separated by a space (like Lip Gloss).
    #[must_use]
    pub fn set_string(mut self, s: impl Into<String>) -> Self {
        self.value = Some(s.into());
        self
    }

    /// Removes the underlying string value.
    #[must_use]
    pub fn unset_string(mut self) -> Self {
        self.value = None;
        self
    }

    /// Returns the underlying string value, if any.
    #[must_use]
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    color_rules! {
        /// Sets the foreground (text) color.
        foreground, unset_foreground, get_foreground;
        /// Sets the background color. By default it also colors padding and
        /// alignment whitespace (see [`Style::color_whitespace`]).
        background, unset_background, get_background;
        /// Sets the underline color.
        underline_color, unset_underline_color, get_underline_color;
        /// Sets the background color of the margins.
        margin_background, unset_margin_background, get_margin_background;
        /// Sets the foreground color of the top border.
        border_top_foreground, unset_border_top_foreground, get_border_top_foreground;
        /// Sets the foreground color of the right border.
        border_right_foreground, unset_border_right_foreground, get_border_right_foreground;
        /// Sets the foreground color of the bottom border.
        border_bottom_foreground, unset_border_bottom_foreground, get_border_bottom_foreground;
        /// Sets the foreground color of the left border.
        border_left_foreground, unset_border_left_foreground, get_border_left_foreground;
        /// Sets the background color of the top border.
        border_top_background, unset_border_top_background, get_border_top_background;
        /// Sets the background color of the right border.
        border_right_background, unset_border_right_background, get_border_right_background;
        /// Sets the background color of the bottom border.
        border_bottom_background, unset_border_bottom_background, get_border_bottom_background;
        /// Sets the background color of the left border.
        border_left_background, unset_border_left_background, get_border_left_background;
    }

    bool_rules! {
        /// Sets the bold attribute.
        bold, unset_bold, get_bold;
        /// Sets the italic attribute.
        italic, unset_italic, get_italic;
        /// Sets the underline attribute.
        underline, unset_underline, get_underline;
        /// Sets the strikethrough (crossed out) attribute.
        strikethrough, unset_strikethrough, get_strikethrough;
        /// Sets the faint (dim) attribute.
        faint, unset_faint, get_faint;
        /// Sets the reverse (swap foreground and background) attribute.
        reverse, unset_reverse, get_reverse;
        /// Sets the (slow) blink attribute.
        blink, unset_blink, get_blink;
        /// Sets the hidden (concealed) attribute.
        hidden, unset_hidden, get_hidden;
        /// Forces rendering onto a single line, ignoring padding, borders
        /// and margins. Newlines are removed.
        inline, unset_inline, get_inline;
        /// Whether the background color applies to padding and alignment
        /// whitespace (default `true`).
        color_whitespace, unset_color_whitespace, get_color_whitespace_rule;
        /// Enables or disables the top border.
        border_top, unset_border_top, get_border_top_rule;
        /// Enables or disables the right border.
        border_right, unset_border_right, get_border_right_rule;
        /// Enables or disables the bottom border.
        border_bottom, unset_border_bottom, get_border_bottom_rule;
        /// Enables or disables the left border.
        border_left, unset_border_left, get_border_left_rule;
    }

    u16_rules! {
        /// Sets the width of the block **including padding but excluding
        /// borders and margins** (Lip Gloss semantics). Text is word-wrapped
        /// to fit and shorter lines are padded according to the horizontal
        /// alignment.
        width, unset_width, get_width;
        /// Sets the minimum height of the block, including padding but
        /// excluding borders and margins.
        height, unset_height, get_height;
        /// Truncates the final rendered output (including borders and
        /// margins) to this many columns.
        max_width, unset_max_width, get_max_width;
        /// Truncates the final rendered output (including borders and
        /// margins) to this many rows.
        max_height, unset_max_height, get_max_height;
        /// Sets the top padding.
        padding_top, unset_padding_top, get_padding_top;
        /// Sets the right padding.
        padding_right, unset_padding_right, get_padding_right;
        /// Sets the bottom padding.
        padding_bottom, unset_padding_bottom, get_padding_bottom;
        /// Sets the left padding.
        padding_left, unset_padding_left, get_padding_left;
        /// Sets the top margin.
        margin_top, unset_margin_top, get_margin_top;
        /// Sets the right margin.
        margin_right, unset_margin_right, get_margin_right;
        /// Sets the bottom margin.
        margin_bottom, unset_margin_bottom, get_margin_bottom;
        /// Sets the left margin.
        margin_left, unset_margin_left, get_margin_left;
    }

    /// Sets the number of spaces a tab is converted to (default
    /// [`DEFAULT_TAB_WIDTH`]); `0` removes tabs.
    #[must_use]
    pub fn tab_width(mut self, n: u16) -> Self {
        self.tab_width = Some(n);
        self
    }

    /// Removes the tab width rule.
    #[must_use]
    pub fn unset_tab_width(mut self) -> Self {
        self.tab_width = None;
        self
    }

    /// Returns the tab width.
    #[must_use]
    pub fn get_tab_width(&self) -> u16 {
        self.tab_width.unwrap_or(DEFAULT_TAB_WIDTH)
    }

    /// Returns whether whitespace takes the background color.
    #[must_use]
    pub fn get_color_whitespace(&self) -> bool {
        self.color_whitespace.unwrap_or(true)
    }

    // ----- alignment -------------------------------------------------------

    /// Sets the horizontal alignment of text within the block (alias of
    /// [`Style::align_horizontal`]). Accepts a [`Position`], an `f32` or a
    /// ratatui [`Alignment`](ratatui::layout::Alignment).
    #[must_use]
    pub fn align(self, pos: impl Into<Position>) -> Self {
        self.align_horizontal(pos)
    }

    /// Sets the horizontal alignment.
    #[must_use]
    pub fn align_horizontal(mut self, pos: impl Into<Position>) -> Self {
        self.align_horizontal = Some(pos.into());
        self
    }

    /// Sets the vertical alignment (used when [`Style::height`] is larger
    /// than the content).
    #[must_use]
    pub fn align_vertical(mut self, pos: impl Into<Position>) -> Self {
        self.align_vertical = Some(pos.into());
        self
    }

    /// Removes the horizontal alignment rule.
    #[must_use]
    pub fn unset_align_horizontal(mut self) -> Self {
        self.align_horizontal = None;
        self
    }

    /// Removes the vertical alignment rule.
    #[must_use]
    pub fn unset_align_vertical(mut self) -> Self {
        self.align_vertical = None;
        self
    }

    /// Returns the horizontal alignment (left if unset).
    #[must_use]
    pub fn get_align_horizontal(&self) -> Position {
        self.align_horizontal.unwrap_or(Position::LEFT)
    }

    /// Returns the vertical alignment (top if unset).
    #[must_use]
    pub fn get_align_vertical(&self) -> Position {
        self.align_vertical.unwrap_or(Position::TOP)
    }

    // ----- padding & margin -----------------------------------------------

    /// Sets padding using CSS shorthand: `2`, `(v, h)`, `(t, h, b)` or
    /// `(t, r, b, l)`.
    #[must_use]
    pub fn padding(mut self, sides: impl Into<Sides>) -> Self {
        let s = sides.into();
        self.padding_top = Some(s.top);
        self.padding_right = Some(s.right);
        self.padding_bottom = Some(s.bottom);
        self.padding_left = Some(s.left);
        self
    }

    /// Removes all padding rules.
    #[must_use]
    pub fn unset_padding(mut self) -> Self {
        self.padding_top = None;
        self.padding_right = None;
        self.padding_bottom = None;
        self.padding_left = None;
        self
    }

    /// Returns the padding on all sides.
    #[must_use]
    pub fn get_padding(&self) -> Sides {
        Sides::new(
            self.get_padding_top(),
            self.get_padding_right(),
            self.get_padding_bottom(),
            self.get_padding_left(),
        )
    }

    /// Sets the character used to fill padding (default `' '`).
    #[must_use]
    pub fn padding_char(mut self, c: char) -> Self {
        self.padding_char = Some(c);
        self
    }

    /// Sets margins using CSS shorthand: `2`, `(v, h)`, `(t, h, b)` or
    /// `(t, r, b, l)`.
    #[must_use]
    pub fn margin(mut self, sides: impl Into<Sides>) -> Self {
        let s = sides.into();
        self.margin_top = Some(s.top);
        self.margin_right = Some(s.right);
        self.margin_bottom = Some(s.bottom);
        self.margin_left = Some(s.left);
        self
    }

    /// Removes all margin rules.
    #[must_use]
    pub fn unset_margins(mut self) -> Self {
        self.margin_top = None;
        self.margin_right = None;
        self.margin_bottom = None;
        self.margin_left = None;
        self
    }

    /// Returns the margins on all sides.
    #[must_use]
    pub fn get_margin(&self) -> Sides {
        Sides::new(
            self.get_margin_top(),
            self.get_margin_right(),
            self.get_margin_bottom(),
            self.get_margin_left(),
        )
    }

    /// Sets the character used to fill margins (default `' '`).
    #[must_use]
    pub fn margin_char(mut self, c: char) -> Self {
        self.margin_char = Some(c);
        self
    }

    // ----- borders ---------------------------------------------------------

    /// Sets the border and enables it on all four sides.
    #[must_use]
    pub fn border(self, border: Border) -> Self {
        self.border_with_sides(border, true)
    }

    /// Sets the border and the sides to draw using CSS shorthand:
    /// `true`, `(vertical, horizontal)`, `(top, horizontal, bottom)` or
    /// `(top, right, bottom, left)`.
    ///
    /// ```
    /// use ratatui_lipgloss::{Border, Style};
    /// // a thick border on top and bottom only
    /// let s = Style::new().border_with_sides(Border::thick(), (true, false));
    /// assert!(s.get_border_sides().top && !s.get_border_sides().left);
    /// ```
    #[must_use]
    pub fn border_with_sides(mut self, border: Border, sides: impl Into<BorderSides>) -> Self {
        let s = sides.into();
        self.border_style = Some(border);
        self.border_top = Some(s.top);
        self.border_right = Some(s.right);
        self.border_bottom = Some(s.bottom);
        self.border_left = Some(s.left);
        self
    }

    /// Sets the border characters without changing which sides are enabled.
    /// If no side was set explicitly, all four sides are drawn.
    #[must_use]
    pub fn border_style(mut self, border: Border) -> Self {
        self.border_style = Some(border);
        self
    }

    /// Removes the border style rule.
    #[must_use]
    pub fn unset_border_style(mut self) -> Self {
        self.border_style = None;
        self
    }

    /// Returns the border style, if set.
    #[must_use]
    pub fn get_border_style(&self) -> Option<Border> {
        self.border_style
    }

    /// Returns the sides on which a border is effectively drawn.
    ///
    /// If a border style is set and no side was set explicitly, all sides
    /// are drawn. If sides are enabled but no style is set,
    /// [`Border::normal`] is used.
    #[must_use]
    pub fn get_border_sides(&self) -> BorderSides {
        let sides = [
            self.border_top,
            self.border_right,
            self.border_bottom,
            self.border_left,
        ];
        if sides.iter().all(Option::is_none) {
            return BorderSides::all(self.border_style.is_some());
        }
        let [t, r, b, l] = sides.map(|s| s.unwrap_or(false));
        BorderSides::new(t, r, b, l)
    }

    /// Sets the border foreground on every side.
    #[must_use]
    pub fn border_foreground(self, c: impl Into<TerminalColor>) -> Self {
        let c = c.into();
        self.border_top_foreground(c)
            .border_right_foreground(c)
            .border_bottom_foreground(c)
            .border_left_foreground(c)
    }

    /// Sets the border background on every side.
    #[must_use]
    pub fn border_background(self, c: impl Into<TerminalColor>) -> Self {
        let c = c.into();
        self.border_top_background(c)
            .border_right_background(c)
            .border_bottom_background(c)
            .border_left_background(c)
    }

    /// Removes every border rule (style, sides and colors).
    #[must_use]
    pub fn unset_border(mut self) -> Self {
        self.border_style = None;
        self.border_top = None;
        self.border_right = None;
        self.border_bottom = None;
        self.border_left = None;
        self.border_top_foreground = None;
        self.border_right_foreground = None;
        self.border_bottom_foreground = None;
        self.border_left_foreground = None;
        self.border_top_background = None;
        self.border_right_background = None;
        self.border_bottom_background = None;
        self.border_left_background = None;
        self
    }

    pub(crate) fn border_spec(&self, dark: bool) -> BorderSpec {
        let sides = self.get_border_sides();
        let style = |fg: Option<TerminalColor>, bg: Option<TerminalColor>| RStyle {
            fg: fg.and_then(|c| c.resolve(dark)),
            bg: bg.and_then(|c| c.resolve(dark)),
            ..RStyle::default()
        };
        BorderSpec {
            border: self.border_style.unwrap_or_else(Border::normal),
            top: sides.top,
            right: sides.right,
            bottom: sides.bottom,
            left: sides.left,
            top_style: style(self.border_top_foreground, self.border_top_background),
            right_style: style(self.border_right_foreground, self.border_right_background),
            bottom_style: style(self.border_bottom_foreground, self.border_bottom_background),
            left_style: style(self.border_left_foreground, self.border_left_background),
        }
    }

    // ----- frame sizes -----------------------------------------------------

    /// Total width of the left and right borders.
    #[must_use]
    pub fn get_horizontal_border_size(&self) -> u16 {
        u16::try_from(self.border_spec(true).horizontal_size()).unwrap_or(u16::MAX)
    }

    /// Total height of the top and bottom borders.
    #[must_use]
    pub fn get_vertical_border_size(&self) -> u16 {
        u16::try_from(self.border_spec(true).vertical_size()).unwrap_or(u16::MAX)
    }

    /// Horizontal margins + borders + padding.
    #[must_use]
    pub fn get_horizontal_frame_size(&self) -> u16 {
        let (p, m) = (self.get_padding(), self.get_margin());
        p.left
            .saturating_add(p.right)
            .saturating_add(m.left)
            .saturating_add(m.right)
            .saturating_add(self.get_horizontal_border_size())
    }

    /// Vertical margins + borders + padding.
    #[must_use]
    pub fn get_vertical_frame_size(&self) -> u16 {
        let (p, m) = (self.get_padding(), self.get_margin());
        p.top
            .saturating_add(p.bottom)
            .saturating_add(m.top)
            .saturating_add(m.bottom)
            .saturating_add(self.get_vertical_border_size())
    }

    // ----- composition -----------------------------------------------------

    /// Copies every rule that is set on `other` but unset on `self`.
    ///
    /// Like Lip Gloss, margins, padding and the underlying string are *not*
    /// inherited. If `other` has a background and `self` has neither a
    /// background nor a margin background, the margin background is
    /// inherited from it.
    ///
    /// ```
    /// use ratatui::style::Color;
    /// use ratatui_lipgloss::{Style, TerminalColor};
    ///
    /// let a = Style::new().foreground(229u8).background(63u8);
    /// let b = Style::new().foreground(201u8).inherit(&a);
    /// assert_eq!(b.get_foreground(), Some(TerminalColor::Color(Color::Indexed(201))));
    /// assert_eq!(b.get_background(), Some(TerminalColor::Color(Color::Indexed(63))));
    /// ```
    #[must_use]
    pub fn inherit(mut self, other: &Self) -> Self {
        if self.background.is_none()
            && self.margin_background.is_none()
            && let Some(bg) = other.background
        {
            self.margin_background = Some(bg);
        }
        macro_rules! inherit {
            ($($f:ident),*) => {$(
                if self.$f.is_none() {
                    self.$f = other.$f;
                }
            )*};
        }
        inherit!(
            foreground,
            background,
            underline_color,
            bold,
            italic,
            underline,
            strikethrough,
            faint,
            reverse,
            blink,
            hidden,
            width,
            height,
            max_width,
            max_height,
            align_horizontal,
            align_vertical,
            margin_background,
            border_style,
            border_top,
            border_right,
            border_bottom,
            border_left,
            border_top_foreground,
            border_right_foreground,
            border_bottom_foreground,
            border_left_foreground,
            border_top_background,
            border_right_background,
            border_bottom_background,
            border_left_background,
            inline,
            tab_width,
            color_whitespace
        );
        self
    }

    // ----- conversions -----------------------------------------------------

    /// Converts the inline (text) rules to a ratatui style, resolving
    /// adaptive colors for the given background.
    ///
    /// Rules explicitly set to `false` become *removed* modifiers, so they
    /// override inherited attributes when patched onto other styles.
    #[must_use]
    pub fn to_ratatui_with(&self, dark_background: bool) -> RStyle {
        let mut s = RStyle {
            fg: self.foreground.and_then(|c| c.resolve(dark_background)),
            bg: self.background.and_then(|c| c.resolve(dark_background)),
            underline_color: self
                .underline_color
                .and_then(|c| c.resolve(dark_background)),
            ..RStyle::default()
        };
        let attrs = [
            (self.bold, Modifier::BOLD),
            (self.italic, Modifier::ITALIC),
            (self.underline, Modifier::UNDERLINED),
            (self.strikethrough, Modifier::CROSSED_OUT),
            (self.faint, Modifier::DIM),
            (self.reverse, Modifier::REVERSED),
            (self.blink, Modifier::SLOW_BLINK),
            (self.hidden, Modifier::HIDDEN),
        ];
        for (rule, modifier) in attrs {
            match rule {
                Some(true) => s = s.add_modifier(modifier),
                Some(false) => s = s.remove_modifier(modifier),
                None => {}
            }
        }
        s
    }

    /// Converts the inline (text) rules to a ratatui style using the global
    /// [`has_dark_background`] setting.
    #[must_use]
    pub fn to_ratatui(&self) -> RStyle {
        self.to_ratatui_with(has_dark_background())
    }

    /// Style used for padding / alignment whitespace.
    pub(crate) fn whitespace_style(&self, dark: bool) -> RStyle {
        if !self.get_color_whitespace() {
            return RStyle::default();
        }
        let mut s = RStyle {
            bg: self.background.and_then(|c| c.resolve(dark)),
            ..RStyle::default()
        };
        if self.get_reverse() {
            s.fg = self.foreground.and_then(|c| c.resolve(dark));
            s = s.add_modifier(Modifier::REVERSED);
        }
        s
    }

    /// Patches this style's inline rules onto anything ratatui can style
    /// (`Span`, `Line`, `Text`, `Paragraph`, `Block`, ...), keeping the
    /// target's existing rules where this style leaves them unset.
    ///
    /// ```
    /// use ratatui::text::Span;
    /// use ratatui::style::{Color, Modifier};
    /// use ratatui_lipgloss::Style;
    ///
    /// let span = Style::new().bold(true).apply(Span::raw("hi").fg(Color::Red));
    /// # use ratatui::style::Stylize;
    /// assert_eq!(span.style.fg, Some(Color::Red));
    /// assert!(span.style.add_modifier.contains(Modifier::BOLD));
    /// ```
    #[must_use]
    pub fn apply<T: Styled<Item = T>>(&self, target: T) -> T {
        let s = target.style().patch(self.to_ratatui());
        target.set_style(s)
    }

    /// Creates a [`Span`] with this style's inline rules.
    #[must_use]
    pub fn span<'a>(&self, content: impl Into<std::borrow::Cow<'a, str>>) -> Span<'a> {
        Span::styled(content, self.to_ratatui())
    }

    /// Creates a single-[`Line`] with this style's inline rules.
    #[must_use]
    pub fn line<'a>(&self, content: impl Into<std::borrow::Cow<'a, str>>) -> Line<'a> {
        Line::from(self.span(content))
    }

    /// Builds an equivalent ratatui [`Block`]: borders, border style (taken
    /// from the top side), padding and inline style.
    ///
    /// ratatui blocks support a single border color and single-character
    /// edges, so per-side colors and multi-grapheme patterns are
    /// approximated. Use [`Style::wrap`] for full fidelity.
    #[must_use]
    pub fn to_block(&self) -> Block<'static> {
        let dark = has_dark_background();
        let spec = self.border_spec(dark);
        let mut borders = Borders::NONE;
        if spec.top {
            borders |= Borders::TOP;
        }
        if spec.right {
            borders |= Borders::RIGHT;
        }
        if spec.bottom {
            borders |= Borders::BOTTOM;
        }
        if spec.left {
            borders |= Borders::LEFT;
        }
        Block::new()
            .borders(borders)
            .border_set(spec.border.to_border_set())
            .border_style(spec.top_style)
            .padding(self.get_padding().into())
            .style(self.to_ratatui_with(dark))
    }
}

impl From<&Style> for RStyle {
    fn from(s: &Style) -> Self {
        s.to_ratatui()
    }
}

impl From<Style> for RStyle {
    fn from(s: Style) -> Self {
        s.to_ratatui()
    }
}

impl From<RStyle> for Style {
    /// Converts a ratatui style into a Lip Gloss style with the equivalent
    /// inline rules. Added modifiers become `true` rules and removed
    /// modifiers become `false` rules.
    fn from(r: RStyle) -> Self {
        let mut s = Self {
            foreground: r.fg.map(TerminalColor::Color),
            background: r.bg.map(TerminalColor::Color),
            underline_color: r.underline_color.map(TerminalColor::Color),
            ..Self::default()
        };
        let rule = |m: Modifier| {
            if r.add_modifier.contains(m) {
                Some(true)
            } else if r.sub_modifier.contains(m) {
                Some(false)
            } else {
                None
            }
        };
        s.bold = rule(Modifier::BOLD);
        s.italic = rule(Modifier::ITALIC);
        s.underline = rule(Modifier::UNDERLINED);
        s.strikethrough = rule(Modifier::CROSSED_OUT);
        s.faint = rule(Modifier::DIM);
        s.reverse = rule(Modifier::REVERSED);
        s.blink = rule(Modifier::SLOW_BLINK).or(rule(Modifier::RAPID_BLINK));
        s.hidden = rule(Modifier::HIDDEN);
        s
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::*;
    use crate::AdaptiveColor;

    #[test]
    fn set_and_unset() {
        let s = Style::new()
            .bold(true)
            .unset_bold()
            .background(227u8)
            .unset_background();
        assert_eq!(s, Style::new());
        assert!(!s.get_bold());
    }

    #[test]
    fn explicit_false_differs_from_unset() {
        assert_ne!(Style::new().bold(false), Style::new());
        let r = Style::new().bold(false).to_ratatui();
        assert!(r.sub_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn inherit_rules() {
        let parent = Style::new()
            .foreground(Color::Red)
            .background(Color::Blue)
            .padding(3)
            .margin(2)
            .italic(true);
        let child = Style::new().foreground(Color::Green).inherit(&parent);
        assert_eq!(child.get_foreground(), Some(Color::Green.into()));
        assert_eq!(child.get_background(), Some(Color::Blue.into()));
        assert!(child.get_italic());
        assert_eq!(child.get_padding(), Sides::all(0));
        assert_eq!(child.get_margin(), Sides::all(0));
        assert_eq!(child.get_margin_background(), Some(Color::Blue.into()));
    }

    #[test]
    fn ratatui_roundtrip() {
        let r = RStyle::default()
            .fg(Color::Red)
            .bg(Color::Indexed(5))
            .add_modifier(Modifier::BOLD | Modifier::ITALIC)
            .remove_modifier(Modifier::DIM);
        let s = Style::from(r);
        assert!(s.get_bold() && s.get_italic());
        assert_eq!(s.faint, Some(false));
        assert_eq!(RStyle::from(&s), r);
    }

    #[test]
    fn adaptive_to_ratatui() {
        let s = Style::new().foreground(AdaptiveColor::new(Color::Black, Color::White));
        assert_eq!(s.to_ratatui_with(true).fg, Some(Color::White));
        assert_eq!(s.to_ratatui_with(false).fg, Some(Color::Black));
    }

    #[test]
    fn border_sides_implicit() {
        assert_eq!(Style::new().get_border_sides(), BorderSides::all(false));
        assert_eq!(
            Style::new()
                .border_style(Border::rounded())
                .get_border_sides(),
            BorderSides::all(true)
        );
        let s = Style::new()
            .border_style(Border::rounded())
            .border_top(true);
        assert_eq!(
            s.get_border_sides(),
            BorderSides::new(true, false, false, false)
        );
        let s = Style::new().border_with_sides(Border::double(), (true, false, false, true));
        assert_eq!(
            s.get_border_sides(),
            BorderSides::new(true, false, false, true)
        );
    }

    #[test]
    fn frame_sizes() {
        let s = Style::new()
            .padding((1, 2))
            .margin((0, 3))
            .border(Border::normal());
        assert_eq!(s.get_horizontal_frame_size(), 2 + 2 + 3 + 3 + 2);
        assert_eq!(s.get_vertical_frame_size(), 1 + 1 + 2);
    }

    #[test]
    fn to_block_has_borders() {
        let b = Style::new().border(Border::rounded()).padding(1);
        let area = ratatui::layout::Rect::new(0, 0, 10, 5);
        assert_eq!(
            b.to_block().inner(area),
            ratatui::layout::Rect::new(2, 2, 6, 1)
        );
    }
}
