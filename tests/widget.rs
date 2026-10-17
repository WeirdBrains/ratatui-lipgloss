//! Tests for wrapping ratatui widgets with a `Style`.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style as RStyle};
use ratatui::widgets::{List, ListState, Paragraph, StatefulWidget, Widget};
use ratatui_lipgloss::{Border, Position, Style, WidgetStyleExt};

#[test]
fn paragraph_inside_border_and_padding() {
    let style = Style::new().border(Border::rounded()).padding((0, 1));
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 4));
    Paragraph::new("hello\nworld")
        .styled(&style)
        .render(buf.area, &mut buf);
    assert_eq!(
        buf,
        Buffer::with_lines(["╭────────╮", "│ hello  │", "│ world  │", "╰────────╯",])
    );
}

#[test]
fn margins_leave_cells_untouched_by_default() {
    let style = Style::new().margin(1).border(Border::normal());
    let mut buf = Buffer::with_lines(["xxxxx", "xxxxx", "xxxxx", "xxxxx"]);
    Paragraph::new("").styled(&style).render(buf.area, &mut buf);
    assert_eq!(
        buf,
        Buffer::with_lines(["xxxxx", "x┌─┐x", "x└─┘x", "xxxxx"])
    );
}

#[test]
fn margin_background_paints_margins() {
    let style = Style::new()
        .margin((0, 1))
        .margin_background(Color::Red)
        .border(Border::ascii());
    let mut buf = Buffer::empty(Rect::new(0, 0, 5, 3));
    style.wrap(Paragraph::new("x")).render(buf.area, &mut buf);
    let mut expected = Buffer::with_lines([" +-+ ", " |x| ", " +-+ "]);
    expected.set_style(Rect::new(0, 0, 1, 3), RStyle::new().bg(Color::Red));
    expected.set_style(Rect::new(4, 0, 1, 3), RStyle::new().bg(Color::Red));
    assert_eq!(buf, expected);
}

#[test]
fn fixed_size_box_is_aligned_in_area() {
    let style = Style::new()
        .border(Border::normal())
        .width(3)
        .height(1)
        .align(Position::CENTER)
        .align_vertical(Position::BOTTOM);
    let mut buf = Buffer::empty(Rect::new(0, 0, 9, 4));
    Paragraph::new("abc")
        .styled(&style)
        .render(buf.area, &mut buf);
    assert_eq!(
        buf,
        Buffer::with_lines(["         ", "  ┌───┐  ", "  │abc│  ", "  └───┘  ",])
    );
}

#[test]
fn inline_style_is_inherited_by_widget() {
    let style = Style::new()
        .background(Color::Blue)
        .foreground(Color::White)
        .bold(true)
        .padding((0, 1));
    let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
    Paragraph::new("ab")
        .styled(&style)
        .render(buf.area, &mut buf);
    // padding cell: background only
    assert_eq!(buf[(0, 0)].bg, Color::Blue);
    // content cell: full style
    let c = &buf[(1, 0)];
    assert_eq!((c.symbol(), c.fg, c.bg), ("a", Color::White, Color::Blue));
    assert!(c.modifier.contains(Modifier::BOLD));
}

#[test]
fn per_side_border_colors_in_buffer() {
    let style = Style::new()
        .border(Border::thick())
        .border_foreground(Color::Green)
        .border_right_foreground(Color::Red);
    let mut buf = Buffer::empty(Rect::new(0, 0, 3, 3));
    Paragraph::new("x")
        .styled(&style)
        .render(buf.area, &mut buf);
    assert_eq!(buf[(0, 1)].fg, Color::Green);
    assert_eq!(buf[(2, 1)].fg, Color::Red);
    assert_eq!(buf[(1, 0)].fg, Color::Green);
    assert_eq!(buf[(1, 0)].symbol(), "━");
}

#[test]
fn stateful_list_is_supported() {
    let style = Style::new().border(Border::normal());
    let list = List::new(["one", "two"]).highlight_symbol(">");
    let mut state = ListState::default().with_selected(Some(1));
    let mut buf = Buffer::empty(Rect::new(0, 0, 6, 4));
    StatefulWidget::render(style.wrap(list), buf.area, &mut buf, &mut state);
    assert_eq!(
        buf,
        Buffer::with_lines(["┌────┐", "│ one│", "│>two│", "└────┘"])
    );
}

#[test]
fn render_by_reference() {
    let styled = Style::new()
        .border(Border::ascii())
        .wrap(Paragraph::new("r"));
    let mut buf = Buffer::empty(Rect::new(0, 0, 3, 3));
    (&styled).render(buf.area, &mut buf);
    assert_eq!(buf, Buffer::with_lines(["+-+", "|r|", "+-+"]));
}

#[test]
fn tiny_areas_do_not_panic() {
    let style = Style::new().border(Border::double()).padding(2).margin(3);
    for (w, h) in [(0, 0), (1, 1), (2, 1), (3, 2), (5, 5)] {
        let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
        Paragraph::new("text")
            .styled(&style)
            .render(buf.area, &mut buf);
    }
    // an area extending past the buffer is clipped to the buffer first
    let mut buf = Buffer::empty(Rect::new(0, 0, 3, 3));
    Paragraph::new("text")
        .styled(&Style::new().border(Border::normal()))
        .render(Rect::new(1, 1, 10, 10), &mut buf);
    assert_eq!(buf, Buffer::with_lines(["   ", " ┌┐", " └┘"]));
}

#[test]
fn layout_and_inner() {
    let style = Style::new()
        .margin(1)
        .border(Border::normal())
        .padding((1, 2));
    let area = Rect::new(0, 0, 20, 10);
    let l = style.layout(area);
    assert_eq!(l.outer, Rect::new(1, 1, 18, 8));
    assert_eq!(l.border, Rect::new(1, 1, 18, 8));
    assert_eq!(l.padding, Rect::new(2, 2, 16, 6));
    assert_eq!(l.content, Rect::new(4, 3, 12, 4));
    assert_eq!(style.inner(area), l.content);
}

#[test]
fn works_in_a_terminal_frame() {
    let mut terminal = Terminal::new(TestBackend::new(8, 3)).unwrap();
    let style = Style::new()
        .border(Border::rounded())
        .border_foreground(Color::Magenta);
    terminal
        .draw(|f| f.render_widget(Paragraph::new("hey").styled(&style), f.area()))
        .unwrap();
    let mut expected = Buffer::with_lines(["╭──────╮", "│hey   │", "╰──────╯"]);
    let m = RStyle::new().fg(Color::Magenta);
    expected.set_style(Rect::new(0, 0, 8, 1), m);
    expected.set_style(Rect::new(0, 2, 8, 1), m);
    expected.set_style(Rect::new(0, 1, 1, 1), m);
    expected.set_style(Rect::new(7, 1, 1, 1), m);
    terminal.backend().assert_buffer(&expected);
}
