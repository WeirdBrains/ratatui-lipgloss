//! # ratatui-lipgloss
//!
//! Declarative, chainable, CSS-like styling for [ratatui], inspired by
//! Charm's [Lip Gloss](https://github.com/charmbracelet/lipgloss).
//!
//! ```
//! use ratatui_lipgloss::{Border, Position, Style};
//!
//! let style = Style::new()
//!     .bold(true)
//!     .foreground("#FAFAFA")
//!     .background("#7D56F4")
//!     .padding((1, 4))
//!     .border(Border::rounded())
//!     .border_foreground("63")
//!     .width(22)
//!     .align(Position::CENTER);
//!
//! // A `ratatui::text::Text` you can render as a widget, put in a
//! // `Paragraph`, or compose with `join_horizontal` / `join_vertical` / `place`.
//! let text = style.render("Hello, kitty");
//! assert_eq!(ratatui_lipgloss::size(&text), (24, 5));
//! ```
//!
//! ## Three ways to use a [`Style`]
//!
//! * **As a ratatui style**: `&Style` converts into
//!   [`ratatui::style::Style`], so it works with `Span::styled`,
//!   `Paragraph::style`, `Block::style`, ... and [`Style::apply`] patches it
//!   onto anything implementing ratatui's `Styled`.
//! * **Rendered to `Text`**: [`Style::render`] applies the whole box model
//!   (wrapping, alignment, width/height, padding, borders, margins, max
//!   sizes) and returns an owned [`Text`](ratatui::text::Text).
//!   [`Style::render_ansi`] gives you an escape-coded `String` for printing
//!   outside a TUI.
//! * **Around a widget**: [`Style::wrap`] / [`WidgetStyleExt::styled`]
//!   render any [`Widget`](ratatui::widgets::Widget) or
//!   [`StatefulWidget`](ratatui::widgets::StatefulWidget) inside the style's
//!   margin, border and padding.
//!
//! ## Colors
//!
//! Every color rule accepts `impl Into<TerminalColor>`: Lip Gloss-style
//! strings (`"#7D56F4"`, `"#fff"`, `"86"`, `"magenta"`), a `u8` ANSI 256
//! index, an `(r, g, b)` tuple, a ratatui [`Color`](ratatui::style::Color)
//! or an [`AdaptiveColor`] that switches on [`has_dark_background`].
//!
//! [ratatui]: https://ratatui.rs

#![cfg_attr(docsrs, feature(doc_cfg))]

mod ansi;
mod border;
mod cells;
mod color;
mod join;
mod position;
mod render;
mod style;
mod widget;

pub use ansi::{to_ansi_string, to_plain_string};
pub use border::{Border, BorderSides};
pub use color::{
    AdaptiveColor, TerminalColor, blend, blend_1d, color, complementary, darken,
    detect_dark_background, has_dark_background, light_dark, lighten, parse_color,
    set_dark_background, to_rgb,
};
pub use join::{
    Whitespace, height, join_horizontal, join_vertical, place, place_horizontal, place_vertical,
    place_with, size, width,
};
pub use position::{Position, Sides};
pub use style::{DEFAULT_TAB_WIDTH, Style};
pub use widget::{BoxLayout, StyledWidget, WidgetStyleExt};

/// Convenient glob import: `use ratatui_lipgloss::prelude::*;`
pub mod prelude {
    pub use crate::{
        AdaptiveColor, Border, Position, Style, StyledWidget, TerminalColor, WidgetStyleExt,
        join_horizontal, join_vertical, place,
    };
}

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
