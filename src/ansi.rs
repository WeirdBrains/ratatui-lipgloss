//! Serializing ratatui [`Text`] to strings.

use core::fmt::Write;

use ratatui::style::{Color, Modifier, Style as RStyle};
use ratatui::text::Text;

/// Returns the characters of `text`, one line per row, without styling.
#[must_use]
pub fn to_plain_string(text: &Text<'_>) -> String {
    text.lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Serializes `text` to a string with ANSI SGR escape codes, so it can be
/// printed directly to a terminal outside of a ratatui app.
///
/// Each styled span is followed by a reset (`ESC[0m`); lines are separated
/// by `\n` with no trailing newline.
#[must_use]
pub fn to_ansi_string(text: &Text<'_>) -> String {
    let mut out = String::new();
    for (i, line) in text.lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        for span in &line.spans {
            let style = text.style.patch(line.style).patch(span.style);
            let sgr = sgr(style);
            if sgr.is_empty() {
                out.push_str(&span.content);
            } else {
                let _ = write!(out, "\x1b[{sgr}m{}\x1b[0m", span.content);
            }
        }
    }
    out
}

fn color_code(c: Color, base: u8) -> String {
    // base = 30 for foreground, 40 for background.
    let named = |n: u8| (base + n).to_string();
    let bright = |n: u8| (base + 60 + n).to_string();
    match c {
        Color::Reset => (base + 9).to_string(),
        Color::Black => named(0),
        Color::Red => named(1),
        Color::Green => named(2),
        Color::Yellow => named(3),
        Color::Blue => named(4),
        Color::Magenta => named(5),
        Color::Cyan => named(6),
        Color::Gray => named(7),
        Color::DarkGray => bright(0),
        Color::LightRed => bright(1),
        Color::LightGreen => bright(2),
        Color::LightYellow => bright(3),
        Color::LightBlue => bright(4),
        Color::LightMagenta => bright(5),
        Color::LightCyan => bright(6),
        Color::White => bright(7),
        Color::Indexed(i) => format!("{};5;{i}", base + 8),
        Color::Rgb(r, g, b) => format!("{};2;{r};{g};{b}", base + 8),
    }
}

fn sgr(style: RStyle) -> String {
    let mut codes: Vec<String> = Vec::new();
    let m = style.add_modifier;
    for (flag, code) in [
        (Modifier::BOLD, "1"),
        (Modifier::DIM, "2"),
        (Modifier::ITALIC, "3"),
        (Modifier::UNDERLINED, "4"),
        (Modifier::SLOW_BLINK, "5"),
        (Modifier::RAPID_BLINK, "6"),
        (Modifier::REVERSED, "7"),
        (Modifier::HIDDEN, "8"),
        (Modifier::CROSSED_OUT, "9"),
    ] {
        if m.contains(flag) {
            codes.push(code.to_string());
        }
    }
    if let Some(fg) = style.fg {
        codes.push(color_code(fg, 30));
    }
    if let Some(bg) = style.bg {
        codes.push(color_code(bg, 40));
    }
    match style.underline_color {
        Some(Color::Indexed(i)) => codes.push(format!("58;5;{i}")),
        Some(Color::Rgb(r, g, b)) => codes.push(format!("58;2;{r};{g};{b}")),
        Some(c) => {
            if let Some((r, g, b)) = crate::to_rgb(c) {
                codes.push(format!("58;2;{r};{g};{b}"));
            }
        }
        None => {}
    }
    codes.join(";")
}

#[cfg(test)]
mod tests {
    use ratatui::style::Stylize;
    use ratatui::text::{Line, Span};

    use super::*;

    #[test]
    fn ansi_output() {
        let text = Text::from(vec![
            Line::from(vec![Span::raw("a"), Span::raw("b").bold().red()]),
            Line::from(Span::raw("c").bg(Color::Rgb(1, 2, 3))),
            Line::from(Span::raw("d").fg(Color::Indexed(201))),
        ]);
        assert_eq!(
            to_ansi_string(&text),
            "a\x1b[1;31mb\x1b[0m\n\x1b[48;2;1;2;3mc\x1b[0m\n\x1b[38;5;201md\x1b[0m"
        );
        assert_eq!(to_plain_string(&text), "ab\nc\nd");
    }
}
