//! Rendering tests: `Style::render` output checked through ratatui buffers.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style as RStyle, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Widget;
use ratatui_lipgloss::{
    AdaptiveColor, Border, Position, Style, join_horizontal, join_vertical, place,
    place_horizontal, place_vertical, size, to_plain_string,
};

fn render_to_buffer(text: &Text<'_>) -> Buffer {
    let (w, h) = size(text);
    let mut buf = Buffer::empty(Rect::new(0, 0, w as u16, h as u16));
    text.clone().render(buf.area, &mut buf);
    buf
}

#[test]
fn plain_text_is_unchanged() {
    let t = Style::new().render("hello\nworld!");
    assert_eq!(to_plain_string(&t), "hello \nworld!");
}

#[test]
fn padding_with_background_fills_cells() {
    let style = Style::new().background(Color::Blue).padding((1, 2));
    let buf = render_to_buffer(&style.render("hi"));
    let mut expected = Buffer::with_lines(["      ", "  hi  ", "      "]);
    expected.set_style(expected.area, RStyle::new().bg(Color::Blue));
    assert_eq!(buf, expected);
}

#[test]
fn readme_example_dimensions() {
    let style = Style::new()
        .bold(true)
        .foreground("#FAFAFA")
        .background("#7D56F4")
        .padding_top(2)
        .padding_left(4)
        .width(22);
    let t = style.render("Hello, kitty");
    assert_eq!(size(&t), (22, 3));
    let buf = render_to_buffer(&t);
    let text_cell = &buf[(4, 2)];
    assert_eq!(text_cell.symbol(), "H");
    assert_eq!(text_cell.fg, Color::Rgb(0xfa, 0xfa, 0xfa));
    assert_eq!(text_cell.bg, Color::Rgb(0x7d, 0x56, 0xf4));
    assert!(text_cell.modifier.contains(Modifier::BOLD));
    // padding takes the background but not the bold attribute
    let pad = &buf[(0, 0)];
    assert_eq!(pad.bg, Color::Rgb(0x7d, 0x56, 0xf4));
    assert!(!pad.modifier.contains(Modifier::BOLD));
}

#[test]
fn color_whitespace_false() {
    let style = Style::new()
        .background(Color::Red)
        .padding((0, 1))
        .color_whitespace(false);
    let buf = render_to_buffer(&style.render("x"));
    assert_eq!(buf[(0, 0)].bg, Color::Reset);
    assert_eq!(buf[(1, 0)].bg, Color::Red);
}

#[test]
fn borders_presets() {
    let cases = [
        (Border::normal(), ["┌──┐", "│hi│", "└──┘"]),
        (Border::rounded(), ["╭──╮", "│hi│", "╰──╯"]),
        (Border::thick(), ["┏━━┓", "┃hi┃", "┗━━┛"]),
        (Border::double(), ["╔══╗", "║hi║", "╚══╝"]),
        (Border::ascii(), ["+--+", "|hi|", "+--+"]),
        (Border::block(), ["████", "█hi█", "████"]),
        (Border::hidden(), ["    ", " hi ", "    "]),
    ];
    for (border, expected) in cases {
        let t = Style::new().border(border).render("hi");
        assert_eq!(
            render_to_buffer(&t),
            Buffer::with_lines(expected),
            "{border:?}"
        );
    }
}

#[test]
fn border_sides_and_colors() {
    let style = Style::new()
        .border_with_sides(Border::rounded(), (true, false, false, true))
        .border_top_foreground(Color::Red)
        .border_left_foreground(Color::Green)
        .border_left_background(Color::Black);
    let buf = render_to_buffer(&style.render("ab\ncd"));
    let mut expected = Buffer::with_lines(["╭──", "│ab", "│cd"]);
    expected.set_style(Rect::new(0, 0, 3, 1), RStyle::new().fg(Color::Red));
    expected.set_style(
        Rect::new(0, 1, 1, 2),
        RStyle::new().fg(Color::Green).bg(Color::Black),
    );
    assert_eq!(buf, expected);
}

#[test]
fn border_without_corners() {
    let t = Style::new()
        .border_style(Border::normal())
        .border_top(true)
        .border_bottom(true)
        .render("abc");
    assert_eq!(to_plain_string(&t), "───\nabc\n───");
}

#[test]
fn custom_border_patterns_repeat() {
    let cute = Border {
        top: "._.:*:",
        bottom: "._.:*:",
        left: "|*",
        right: "|*",
        top_left: "*",
        top_right: "*",
        bottom_left: "*",
        bottom_right: "*",
        ..Border::empty()
    };
    let t = Style::new().border(cute).render("12345678\n2\n3");
    assert_eq!(
        to_plain_string(&t),
        "*._.:*:._*\n|12345678|\n*2       *\n|3       |\n*._.:*:._*"
    );
}

#[test]
fn width_wraps_and_aligns() {
    let t = Style::new().width(10).render("the quick brown fox");
    assert_eq!(to_plain_string(&t), "the quick \nbrown fox ");

    let t = Style::new()
        .width(11)
        .align(Position::CENTER)
        .render("the quick brown fox");
    assert_eq!(to_plain_string(&t), " the quick \n brown fox ");

    let t = Style::new()
        .width(8)
        .align(Position::RIGHT)
        .padding((0, 1))
        .render("ab");
    assert_eq!(to_plain_string(&t), "     ab ");
}

#[test]
fn height_and_vertical_alignment() {
    let s = Style::new().height(3).align_vertical(Position::BOTTOM);
    assert_eq!(to_plain_string(&s.render("x")), " \n \nx");
    let s = Style::new()
        .width(3)
        .height(3)
        .align(Position::CENTER)
        .align_vertical(Position::CENTER);
    assert_eq!(to_plain_string(&s.render("x")), "   \n x \n   ");
}

#[test]
fn margins_and_margin_background() {
    let style = Style::new()
        .border(Border::normal())
        .margin((1, 2))
        .margin_background(Color::Yellow);
    let buf = render_to_buffer(&style.render("x"));
    let mut expected = Buffer::with_lines(["       ", "  ┌─┐  ", "  │x│  ", "  └─┘  ", "       "]);
    let y = RStyle::new().bg(Color::Yellow);
    expected.set_style(Rect::new(0, 0, 7, 1), y);
    expected.set_style(Rect::new(0, 4, 7, 1), y);
    for row in 1..4 {
        expected.set_style(Rect::new(0, row, 2, 1), y);
        expected.set_style(Rect::new(5, row, 2, 1), y);
    }
    assert_eq!(buf, expected);
}

#[test]
fn padding_and_margin_chars() {
    let t = Style::new()
        .padding((0, 1))
        .padding_char('·')
        .margin((0, 1))
        .margin_char('░')
        .render("x");
    assert_eq!(to_plain_string(&t), "░·x·░");
}

#[test]
fn max_width_and_height_truncate() {
    let t = Style::new()
        .border(Border::normal())
        .max_width(4)
        .max_height(2)
        .render("hello\nworld");
    assert_eq!(to_plain_string(&t), "┌───\n│hel");
}

#[test]
fn inline_ignores_box_model() {
    let t = Style::new()
        .inline(true)
        .padding(2)
        .border(Border::double())
        .margin(1)
        .render("a\nb");
    assert_eq!(to_plain_string(&t), "ab");
}

#[test]
fn existing_span_styles_win() {
    let content = Line::from(vec![Span::raw("a"), Span::raw("b").fg(Color::Red)]);
    let t = Style::new()
        .foreground(Color::Blue)
        .bold(true)
        .render(content);
    let buf = render_to_buffer(&t);
    assert_eq!(buf[(0, 0)].fg, Color::Blue);
    assert_eq!(buf[(1, 0)].fg, Color::Red);
    assert!(buf[(1, 0)].modifier.contains(Modifier::BOLD));
}

#[test]
fn adaptive_colors_resolve() {
    let s = Style::new().foreground(AdaptiveColor::new(Color::Black, Color::White));
    let dark = s.render_with("x", true);
    let light = s.render_with("x", false);
    assert_eq!(dark.lines[0].spans[0].style.fg, Some(Color::White));
    assert_eq!(light.lines[0].spans[0].style.fg, Some(Color::Black));
}

#[test]
fn set_string_and_display() {
    let s = Style::new().set_string("Hello,");
    assert_eq!(to_plain_string(&s.render("kitty.")), "Hello, kitty.");
    assert_eq!(s.to_string(), "Hello,");
    let bold = Style::new().bold(true).set_string("hi");
    assert_eq!(bold.to_string(), "\x1b[1mhi\x1b[0m");
}

#[test]
fn render_ansi_includes_codes() {
    let s = Style::new().foreground(Color::Indexed(201)).italic(true);
    assert_eq!(s.render_ansi("x"), "\x1b[3;38;5;201mx\x1b[0m");
}

#[test]
fn wide_characters_are_measured() {
    let t = Style::new().border(Border::normal()).render("你好");
    assert_eq!(to_plain_string(&t), "┌────┐\n│你好│\n└────┘");
    assert_eq!(size(&t), (6, 3));
}

#[test]
fn tabs_are_converted() {
    assert_eq!(Style::new().render_plain("a\tb"), "a    b");
    assert_eq!(Style::new().tab_width(1).render_plain("a\tb"), "a b");
}

#[test]
fn empty_content_still_renders_box() {
    let t = Style::new()
        .border(Border::normal())
        .padding((0, 1))
        .render("");
    assert_eq!(to_plain_string(&t), "┌──┐\n│  │\n└──┘");
}

#[test]
fn join_horizontal_positions() {
    let a = "a\na\na";
    let b = "b";
    let top = join_horizontal(Position::TOP, [a, b]);
    assert_eq!(to_plain_string(&top), "ab\na \na ");
    let center = join_horizontal(Position::CENTER, [a, b]);
    assert_eq!(to_plain_string(&center), "a \nab\na ");
    let bottom = join_horizontal(Position::BOTTOM, [a, b]);
    assert_eq!(to_plain_string(&bottom), "a \na \nab");
}

#[test]
fn join_preserves_styles_and_composes_rendered_blocks() {
    let left = Style::new()
        .border(Border::rounded())
        .foreground(Color::Red)
        .render("L");
    let right = Style::new()
        .padding((0, 1))
        .background(Color::Blue)
        .render("R");
    let joined = join_horizontal(Position::TOP, [left, right]);
    assert_eq!(to_plain_string(&joined), "╭─╮ R \n│L│   \n╰─╯   ");
    let buf = render_to_buffer(&joined);
    assert_eq!(buf[(1, 1)].fg, Color::Red);
    assert_eq!(buf[(3, 0)].bg, Color::Blue);
    assert_eq!(buf[(3, 1)].bg, Color::Reset);
}

#[test]
fn join_vertical_positions() {
    assert_eq!(
        to_plain_string(&join_vertical(Position::LEFT, ["abc", "x"])),
        "abc\nx  "
    );
    assert_eq!(
        to_plain_string(&join_vertical(Position::RIGHT, ["abc", "x"])),
        "abc\n  x"
    );
    assert_eq!(
        to_plain_string(&join_vertical(0.5, ["abcd", "x"])),
        "abcd\n x  "
    );
}

#[test]
fn place_helpers() {
    assert_eq!(
        to_plain_string(&place_horizontal(5, Position::CENTER, "x")),
        "  x  "
    );
    assert_eq!(
        to_plain_string(&place_vertical(3, Position::CENTER, "x")),
        " \nx\n "
    );
    assert_eq!(
        to_plain_string(&place(3, 3, Position::CENTER, Position::CENTER, "x")),
        "   \n x \n   "
    );
    // larger content is left as-is
    assert_eq!(
        to_plain_string(&place_horizontal(2, Position::CENTER, "abcd")),
        "abcd"
    );
}

#[test]
fn rendered_text_through_test_backend() {
    let style = Style::new()
        .border(Border::rounded())
        .padding((0, 1))
        .align(Position::CENTER)
        .width(8);
    let backend = TestBackend::new(12, 4);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| f.render_widget(style.render("hello"), f.area()))
        .unwrap();
    terminal.backend().assert_buffer_lines([
        "╭────────╮  ",
        "│ hello  │  ",
        "╰────────╯  ",
        "            ",
    ]);
}

#[test]
fn apply_and_into_ratatui_style() {
    let s = Style::new().underline(true).foreground("#ff0000");
    let span = Span::styled("x", &s);
    assert_eq!(span.style.fg, Some(Color::Rgb(255, 0, 0)));
    let line = s.apply(Line::from("y").bg(Color::Blue));
    assert_eq!(line.style.bg, Some(Color::Blue));
    assert!(line.style.add_modifier.contains(Modifier::UNDERLINED));
    assert_eq!(s.span("z").style, RStyle::from(&s));
}
