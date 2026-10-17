//! Colors, adaptive (light/dark) colors and color utilities.
//!
//! Lip Gloss accepts colors as strings (`"#7D56F4"`, `"86"`, `"5"`). This
//! module provides the same convenience on top of [`ratatui::style::Color`]:
//! every color-taking method on [`Style`](crate::Style) accepts anything that
//! converts into a [`TerminalColor`], including `&str`, `u8` (ANSI 256 index),
//! `(u8, u8, u8)` (true color), a ratatui [`Color`] or an [`AdaptiveColor`].

use core::sync::atomic::{AtomicBool, Ordering};

use ratatui::style::Color;

/// A color as understood by a [`Style`](crate::Style) rule.
///
/// This mirrors Lip Gloss's `TerminalColor` interface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TerminalColor {
    /// The absence of a color: the terminal default is used.
    #[default]
    NoColor,
    /// A concrete ratatui color.
    Color(Color),
    /// A color that depends on whether the terminal background is dark.
    Adaptive(AdaptiveColor),
}

impl TerminalColor {
    /// Resolves this color to a concrete ratatui color.
    ///
    /// Returns `None` for [`TerminalColor::NoColor`].
    #[must_use]
    pub const fn resolve(self, dark_background: bool) -> Option<Color> {
        match self {
            Self::NoColor => None,
            Self::Color(c) => Some(c),
            Self::Adaptive(a) => Some(a.resolve(dark_background)),
        }
    }
}

impl From<Color> for TerminalColor {
    fn from(c: Color) -> Self {
        Self::Color(c)
    }
}

impl From<AdaptiveColor> for TerminalColor {
    fn from(c: AdaptiveColor) -> Self {
        Self::Adaptive(c)
    }
}

impl From<&str> for TerminalColor {
    /// Parses a color string (see [`parse_color`]). Invalid strings become
    /// [`TerminalColor::NoColor`], matching Lip Gloss's lenient behavior.
    fn from(s: &str) -> Self {
        parse_color(s).map_or(Self::NoColor, Self::Color)
    }
}

impl From<String> for TerminalColor {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl From<&String> for TerminalColor {
    fn from(s: &String) -> Self {
        Self::from(s.as_str())
    }
}

impl From<u8> for TerminalColor {
    /// An ANSI 256 color index.
    fn from(i: u8) -> Self {
        Self::Color(Color::Indexed(i))
    }
}

impl From<(u8, u8, u8)> for TerminalColor {
    /// A 24-bit true color.
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::Color(Color::Rgb(r, g, b))
    }
}

impl From<Option<Color>> for TerminalColor {
    fn from(c: Option<Color>) -> Self {
        c.map_or(Self::NoColor, Self::Color)
    }
}

/// A color with a variant for light and one for dark terminal backgrounds.
///
/// Which variant is used is decided at render time from
/// [`has_dark_background`] (or the explicit flag passed to the `*_with`
/// rendering methods).
///
/// ```
/// use ratatui_lipgloss::{AdaptiveColor, Style, color};
///
/// let subtle = AdaptiveColor::new(color("#D9DCCF"), color("#383838"));
/// let style = Style::new().foreground(subtle);
/// # let _ = style;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AdaptiveColor {
    /// Used on light backgrounds.
    pub light: Color,
    /// Used on dark backgrounds.
    pub dark: Color,
}

impl AdaptiveColor {
    /// Creates a new adaptive color.
    #[must_use]
    pub const fn new(light: Color, dark: Color) -> Self {
        Self { light, dark }
    }

    /// Picks the variant for the given background.
    #[must_use]
    pub const fn resolve(self, dark_background: bool) -> Color {
        if dark_background {
            self.dark
        } else {
            self.light
        }
    }
}

/// Parses a color the way Lip Gloss does.
///
/// Accepted forms:
/// * `"#RRGGBB"` and `"#RGB"`: true color,
/// * `"0"`..=`"255"`: ANSI / ANSI 256 index,
/// * any color name understood by ratatui (`"red"`, `"light-blue"`, `"reset"`, ...).
///
/// ```
/// use ratatui::style::Color;
/// use ratatui_lipgloss::parse_color;
///
/// assert_eq!(parse_color("#7D56F4"), Some(Color::Rgb(0x7d, 0x56, 0xf4)));
/// assert_eq!(parse_color("#fff"), Some(Color::Rgb(255, 255, 255)));
/// assert_eq!(parse_color("86"), Some(Color::Indexed(86)));
/// assert_eq!(parse_color("magenta"), Some(Color::Magenta));
/// assert_eq!(parse_color("nope"), None);
/// ```
#[must_use]
pub fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        let digits: Vec<u8> = hex
            .chars()
            .map(|c| c.to_digit(16).and_then(|d| u8::try_from(d).ok()))
            .collect::<Option<_>>()?;
        return match digits.as_slice() {
            [r, g, b] => Some(Color::Rgb(r * 17, g * 17, b * 17)),
            [r1, r2, g1, g2, b1, b2] => Some(Color::Rgb(r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2)),
            _ => None,
        };
    }
    if let Ok(i) = s.parse::<u8>() {
        return Some(Color::Indexed(i));
    }
    s.parse::<Color>().ok()
}

/// Parses a color string, returning [`Color::Reset`] if it is invalid.
///
/// This is the Rust counterpart of Lip Gloss's `lipgloss.Color("...")`.
#[must_use]
pub fn color(s: &str) -> Color {
    parse_color(s).unwrap_or(Color::Reset)
}

static DARK_BACKGROUND: AtomicBool = AtomicBool::new(true);

/// Returns whether adaptive colors should resolve to their dark variant.
///
/// Defaults to `true`. Change it with [`set_dark_background`], e.g. using the
/// result of [`detect_dark_background`].
#[must_use]
pub fn has_dark_background() -> bool {
    DARK_BACKGROUND.load(Ordering::Relaxed)
}

/// Sets whether adaptive colors resolve to their dark variant (global).
pub fn set_dark_background(dark: bool) {
    DARK_BACKGROUND.store(dark, Ordering::Relaxed);
}

/// Best-effort detection of a dark terminal background from the `COLORFGBG`
/// environment variable (set by rxvt, Konsole, iTerm2 and others).
///
/// Returns `None` if the variable is missing or unparseable. This performs no
/// terminal I/O, so it never competes with your event loop for stdin.
#[must_use]
pub fn detect_dark_background() -> Option<bool> {
    let value = std::env::var("COLORFGBG").ok()?;
    dark_from_colorfgbg(&value)
}

pub(crate) fn dark_from_colorfgbg(value: &str) -> Option<bool> {
    let bg: u8 = value.rsplit(';').next()?.trim().parse().ok()?;
    Some(matches!(bg, 0..=6 | 8))
}

/// Returns a chooser that picks the light or dark color, like Lip Gloss v2's
/// `LightDark`.
///
/// ```
/// use ratatui::style::Color;
/// use ratatui_lipgloss::light_dark;
///
/// let pick = light_dark(true);
/// assert_eq!(pick(Color::White, Color::Black), Color::Black);
/// ```
pub fn light_dark(dark_background: bool) -> impl Fn(Color, Color) -> Color {
    move |light, dark| if dark_background { dark } else { light }
}

const ANSI16: [(u8, u8, u8); 16] = [
    (0, 0, 0),
    (128, 0, 0),
    (0, 128, 0),
    (128, 128, 0),
    (0, 0, 128),
    (128, 0, 128),
    (0, 128, 128),
    (192, 192, 192),
    (128, 128, 128),
    (255, 0, 0),
    (0, 255, 0),
    (255, 255, 0),
    (0, 0, 255),
    (255, 0, 255),
    (0, 255, 255),
    (255, 255, 255),
];

/// Converts a color to RGB using the standard xterm palette.
///
/// Returns `None` for [`Color::Reset`].
#[must_use]
pub fn to_rgb(color: Color) -> Option<(u8, u8, u8)> {
    let idx = match color {
        Color::Reset => return None,
        Color::Rgb(r, g, b) => return Some((r, g, b)),
        Color::Black => 0,
        Color::Red => 1,
        Color::Green => 2,
        Color::Yellow => 3,
        Color::Blue => 4,
        Color::Magenta => 5,
        Color::Cyan => 6,
        Color::Gray => 7,
        Color::DarkGray => 8,
        Color::LightRed => 9,
        Color::LightGreen => 10,
        Color::LightYellow => 11,
        Color::LightBlue => 12,
        Color::LightMagenta => 13,
        Color::LightCyan => 14,
        Color::White => 15,
        Color::Indexed(i) => i,
    };
    Some(match idx {
        0..=15 => ANSI16[idx as usize],
        16..=231 => {
            let i = idx - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            (level(i / 36), level((i / 6) % 6), level(i % 6))
        }
        _ => {
            let v = 8 + 10 * (idx - 232);
            (v, v, v)
        }
    })
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    let v = f32::from(a) + (f32::from(b) - f32::from(a)) * t;
    // Clamped to 0..=255, so the cast is lossless.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        v.round().clamp(0.0, 255.0) as u8
    }
}

/// Linearly blends two colors in RGB space; `t` is clamped to `0.0..=1.0`.
///
/// If either color is [`Color::Reset`] the other color is returned.
#[must_use]
pub fn blend(a: Color, b: Color, t: f32) -> Color {
    let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
    match (to_rgb(a), to_rgb(b)) {
        (Some((r1, g1, b1)), Some((r2, g2, b2))) => {
            Color::Rgb(lerp(r1, r2, t), lerp(g1, g2, t), lerp(b1, b2, t))
        }
        (None, _) => b,
        (_, None) => a,
    }
}

/// Produces `steps` colors forming a gradient through all `stops`, like Lip
/// Gloss's `Blend1D`.
///
/// ```
/// use ratatui::style::Color;
/// use ratatui_lipgloss::blend_1d;
///
/// let g = blend_1d(3, &[Color::Rgb(0, 0, 0), Color::Rgb(200, 100, 0)]);
/// assert_eq!(g, vec![Color::Rgb(0, 0, 0), Color::Rgb(100, 50, 0), Color::Rgb(200, 100, 0)]);
/// ```
#[must_use]
pub fn blend_1d(steps: usize, stops: &[Color]) -> Vec<Color> {
    match stops {
        [] => Vec::new(),
        [only] => vec![*only; steps],
        _ => (0..steps)
            .map(|i| {
                #[allow(clippy::cast_precision_loss)]
                let t = if steps <= 1 {
                    0.0
                } else {
                    i as f32 / (steps - 1) as f32
                };
                #[allow(clippy::cast_precision_loss)]
                let segment = t * (stops.len() - 1) as f32;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let idx = (segment.floor() as usize).min(stops.len() - 2);
                #[allow(clippy::cast_precision_loss)]
                let local = segment - idx as f32;
                blend(stops[idx], stops[idx + 1], local)
            })
            .collect(),
    }
}

/// Darkens a color by mixing it with black; `amount` is `0.0..=1.0`.
#[must_use]
pub fn darken(c: Color, amount: f32) -> Color {
    blend(c, Color::Rgb(0, 0, 0), amount)
}

/// Lightens a color by mixing it with white; `amount` is `0.0..=1.0`.
#[must_use]
pub fn lighten(c: Color, amount: f32) -> Color {
    blend(c, Color::Rgb(255, 255, 255), amount)
}

/// Returns the complementary color (hue rotated by 180°).
///
/// ```
/// use ratatui::style::Color;
/// use ratatui_lipgloss::complementary;
///
/// assert_eq!(complementary(Color::Rgb(255, 0, 0)), Color::Rgb(0, 255, 255));
/// ```
#[must_use]
pub fn complementary(c: Color) -> Color {
    let Some((r, g, b)) = to_rgb(c) else {
        return c;
    };
    let (h, s, l) = rgb_to_hsl(r, g, b);
    let (r, g, b) = hsl_to_rgb((h + 180.0) % 360.0, s, l);
    Color::Rgb(r, g, b)
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let (r, g, b) = (
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d == 0.0 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if (max - r).abs() < f32::EPSILON {
        60.0 * (((g - b) / d) % 6.0)
    } else if (max - g).abs() < f32::EPSILON {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    ((h + 360.0) % 360.0, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let to = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    (to(r), to(g), to(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colors() {
        assert_eq!(parse_color("#FAFAFA"), Some(Color::Rgb(250, 250, 250)));
        assert_eq!(parse_color("#0f0"), Some(Color::Rgb(0, 255, 0)));
        assert_eq!(parse_color("201"), Some(Color::Indexed(201)));
        assert_eq!(parse_color("bright-red"), Some(Color::LightRed));
        assert_eq!(parse_color("#12345"), None);
        assert_eq!(color("garbage"), Color::Reset);
        assert_eq!(TerminalColor::from("garbage"), TerminalColor::NoColor);
    }

    #[test]
    fn adaptive_resolution() {
        let a = AdaptiveColor::new(Color::White, Color::Black);
        assert_eq!(TerminalColor::from(a).resolve(true), Some(Color::Black));
        assert_eq!(TerminalColor::from(a).resolve(false), Some(Color::White));
        assert_eq!(TerminalColor::NoColor.resolve(true), None);
    }

    #[test]
    fn colorfgbg() {
        assert_eq!(dark_from_colorfgbg("15;0"), Some(true));
        assert_eq!(dark_from_colorfgbg("0;15"), Some(false));
        assert_eq!(dark_from_colorfgbg("0;default;7"), Some(false));
        assert_eq!(dark_from_colorfgbg("x"), None);
    }

    #[test]
    fn palette_and_utils() {
        assert_eq!(to_rgb(Color::Indexed(16)), Some((0, 0, 0)));
        assert_eq!(to_rgb(Color::Indexed(231)), Some((255, 255, 255)));
        assert_eq!(to_rgb(Color::Indexed(232)), Some((8, 8, 8)));
        assert_eq!(to_rgb(Color::Red), Some((128, 0, 0)));
        assert_eq!(
            darken(Color::Rgb(200, 100, 50), 0.5),
            Color::Rgb(100, 50, 25)
        );
        assert_eq!(lighten(Color::Rgb(0, 0, 0), 1.0), Color::Rgb(255, 255, 255));
        assert_eq!(blend(Color::Reset, Color::Red, 0.5), Color::Red);
        assert_eq!(blend_1d(0, &[Color::Red, Color::Blue]), vec![]);
        assert_eq!(blend_1d(2, &[Color::Red]), vec![Color::Red, Color::Red]);
        let g = blend_1d(
            5,
            &[
                Color::Rgb(0, 0, 0),
                Color::Rgb(100, 0, 0),
                Color::Rgb(100, 100, 0),
            ],
        );
        assert_eq!(g[2], Color::Rgb(100, 0, 0));
        assert_eq!(g[4], Color::Rgb(100, 100, 0));
        assert_eq!(
            complementary(Color::Rgb(0, 0, 255)),
            Color::Rgb(255, 255, 0)
        );
    }
}
