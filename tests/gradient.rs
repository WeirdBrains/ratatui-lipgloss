//! Gradient (blended) border tests.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::{Paragraph, Widget};
use ratatui_lipgloss::{Border, Style, TerminalColor, WidgetStyleExt, blend_1d, to_plain_string};

const RED: Color = Color::Rgb(255, 0, 0);
const BLUE: Color = Color::Rgb(0, 0, 255);

fn gradient_style() -> Style {
    Style::new()
        .border(Border::normal())
        .border_foreground_blend([RED, BLUE])
}

#[test]
fn blends_clockwise_around_the_perimeter() {
    // 2x1 content: perimeter of (2 + 1 + 2) * 2 = 10 colors.
    let text = gradient_style().render("ab");
    assert_eq!(to_plain_string(&text), "┌──┐\n│ab│\n└──┘");
    let mut buf = Buffer::empty(Rect::new(0, 0, 4, 3));
    text.render(buf.area, &mut buf);
    let g = blend_1d(10, &[RED, BLUE]);
    // top: left to right
    for x in 0..4 {
        assert_eq!(buf[(x, 0)].fg, g[usize::from(x)]);
    }
    // right side
    assert_eq!(buf[(3, 1)].fg, g[4]);
    // bottom: drawn right to left
    for x in 0..4 {
        assert_eq!(buf[(x, 2)].fg, g[5 + 3 - usize::from(x)]);
    }
    // left side
    assert_eq!(buf[(0, 1)].fg, g[9]);
    // content is untouched
    assert_eq!(buf[(1, 1)].fg, Color::Reset);
}

#[test]
fn offset_rotates_the_gradient() {
    let g = blend_1d(10, &[RED, BLUE]);
    let rotated = gradient_style()
        .border_foreground_blend_offset(1)
        .render("ab");
    let rotated_copy = rotated.clone();
    let mut buf = Buffer::empty(Rect::new(0, 0, 4, 3));
    rotated_copy.render(buf.area, &mut buf);
    assert_eq!(buf[(0, 0)].fg, g[9]);
    assert_eq!(buf[(1, 0)].fg, g[0]);
    // negative and oversized offsets wrap around
    let a = gradient_style()
        .border_foreground_blend_offset(-1)
        .render("ab");
    let b = gradient_style()
        .border_foreground_blend_offset(9)
        .render("ab");
    assert_eq!(a, b);
    let c = gradient_style()
        .border_foreground_blend_offset(11)
        .render("ab");
    assert_eq!(c, rotated);
}

#[test]
fn widget_and_ansi_use_the_gradient() {
    let style = gradient_style().padding((0, 1));
    let mut from_widget = Buffer::empty(Rect::new(0, 0, 6, 3));
    Paragraph::new("ab")
        .styled(&style)
        .render(from_widget.area, &mut from_widget);
    let mut from_text = Buffer::empty(Rect::new(0, 0, 6, 3));
    style.render("ab").render(from_text.area, &mut from_text);
    assert_eq!(from_widget, from_text);
    let ansi = style.render_ansi("ab");
    assert!(ansi.contains("38;2;255;0;0"));
}

#[test]
fn setter_semantics() {
    // no colors: no-op; one color: plain border foreground
    assert_eq!(
        Style::new().border_foreground_blend(Vec::<Color>::new()),
        Style::new()
    );
    assert_eq!(
        Style::new().border_foreground_blend([RED]),
        Style::new().border_foreground(RED)
    );
    let s = Style::new()
        .border_foreground_blend(["#ff0000", "#0000ff"])
        .border_foreground_blend_offset(3);
    assert_eq!(
        s.get_border_foreground_blend(),
        Some(&[TerminalColor::Color(RED), TerminalColor::Color(BLUE)][..])
    );
    assert_eq!(s.get_border_foreground_blend_offset(), 3);
    let inherited = Style::new().inherit(&s);
    assert_eq!(inherited.get_border_foreground_blend_offset(), 3);
    assert!(inherited.get_border_foreground_blend().is_some());
    let cleared = s
        .clone()
        .unset_border_foreground_blend()
        .unset_border_foreground_blend_offset();
    assert_eq!(cleared, Style::new());
    assert_eq!(s.unset_border(), Style::new());
}

#[test]
fn partial_borders_and_background_are_kept() {
    let s = Style::new()
        .border_with_sides(Border::rounded(), (true, false))
        .border_background(Color::Black)
        .border_foreground_blend([RED, BLUE, RED]);
    let text = s.render("abc");
    assert_eq!(to_plain_string(&text), "───\nabc\n───");
    let mut buf = Buffer::empty(Rect::new(0, 0, 3, 3));
    text.render(buf.area, &mut buf);
    assert_eq!(buf[(0, 0)].bg, Color::Black);
    assert_ne!(buf[(0, 0)].fg, buf[(2, 0)].fg);
}
