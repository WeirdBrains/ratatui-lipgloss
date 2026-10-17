//! Table tests.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::widgets::Widget;
use ratatui_lipgloss::table::{Table, TableRow};
use ratatui_lipgloss::{Border, Position, Style, size, to_plain_string};

fn cell_style(row: TableRow, _col: usize) -> Style {
    match row {
        TableRow::Header => Style::new().padding((0, 1)).align(Position::CENTER),
        TableRow::Data(_) => Style::new().padding((0, 1)),
    }
}

fn languages() -> Table {
    Table::new()
        .style_func(cell_style)
        .headers(["LANGUAGE", "FORMAL", "INFORMAL"])
        .row(["Chinese", "Nǐn hǎo", "Nǐ hǎo"])
        .row(["French", "Bonjour", "Salut"])
        .row(["Japanese", "こんにちは", "やあ"])
        .rows([
            ["Russian", "Zdravstvuyte", "Privet"],
            ["Spanish", "Hola", "¿Qué tal?"],
        ])
}

const FULL: &str = "\
┌──────────┬──────────────┬───────────┐
│ LANGUAGE │    FORMAL    │ INFORMAL  │
├──────────┼──────────────┼───────────┤
│ Chinese  │ Nǐn hǎo      │ Nǐ hǎo    │
│ French   │ Bonjour      │ Salut     │
│ Japanese │ こんにちは   │ やあ      │
│ Russian  │ Zdravstvuyte │ Privet    │
│ Spanish  │ Hola         │ ¿Qué tal? │
└──────────┴──────────────┴───────────┘";

#[test]
fn default_table_matches_lipgloss() {
    assert_eq!(languages().to_string(), FULL);
}

#[test]
fn double_border_and_inner_borders_only() {
    let t = languages().border(Border::double()).to_string();
    assert!(t.starts_with("╔══════════╦"));
    assert!(t.contains("╠══════════╬"));
    let inner = languages()
        .border_top(false)
        .border_bottom(false)
        .border_left(false)
        .border_right(false)
        .border_row(true)
        .to_string();
    let lines: Vec<&str> = inner.lines().collect();
    assert_eq!(lines[0], " LANGUAGE │    FORMAL    │ INFORMAL  ");
    assert_eq!(lines[1], "──────────┼──────────────┼───────────");
    assert_eq!(lines[3], "──────────┼──────────────┼───────────");
    assert_eq!(lines.len(), 11);
}

#[test]
fn toggles_have_getters() {
    let t = Table::new()
        .border_row(true)
        .border_column(false)
        .wrap(false);
    assert!(t.get_border_row() && !t.get_border_column() && !t.get_wrap());
    assert!(t.get_border_top() && t.get_border_bottom() && t.get_border_header());
    assert!(t.get_border_left() && t.get_border_right());
}

#[test]
fn height_overflow_and_offset() {
    let t = languages().height(6);
    assert_eq!(
        t.to_string(),
        "┌──────────┬──────────────┬───────────┐\n\
         │ LANGUAGE │    FORMAL    │ INFORMAL  │\n\
         ├──────────┼──────────────┼───────────┤\n\
         │ Chinese  │ Nǐn hǎo      │ Nǐ hǎo    │\n\
         │ …        │ …            │ …         │\n\
         └──────────┴──────────────┴───────────┘"
    );
    assert_eq!(t.visible_rows(), 0..1);
    let scrolled = languages().height(8).y_offset(1);
    assert_eq!(scrolled.visible_rows(), 1..5);
    assert!(!scrolled.to_string().contains("Chinese"));
    assert_eq!(scrolled.get_y_offset(), 1);
    assert_eq!(scrolled.get_height(), Some(8));
    // tiny heights clip like Lip Gloss
    assert_eq!(
        languages().height(1).to_string(),
        FULL.lines().next().unwrap()
    );
    assert_eq!(languages().height(4).to_string().lines().count(), 4);
    // separators between rows count against the height
    let with_rows = languages().border_row(true).height(8);
    assert_eq!(with_rows.visible_rows(), 0..1);
    assert!(with_rows.to_string().contains("│ …"));
}

#[test]
fn width_expands_and_shrinks() {
    let wide = languages().width(80);
    assert_eq!(wide.get_width(), Some(80));
    assert_eq!(size(&wide.render()).0, 80);
    let narrow = languages().width(30).render();
    assert_eq!(size(&narrow).0, 30);
    // long cells wrap onto several lines
    assert!(narrow.lines.len() > FULL.lines().count());
    // without wrapping they are truncated with an ellipsis
    let truncated = languages().width(30).wrap(false).to_string();
    assert_eq!(truncated.lines().count(), FULL.lines().count());
    assert!(truncated.contains('…'));
}

#[test]
fn style_func_base_style_and_border_style() {
    let t = Table::new()
        .headers(["A", "B"])
        .row(["1", "2"])
        .row(["3", "4"])
        .base_style(Style::new().background(Color::Blue))
        .border_style(Style::new().foreground(Color::Red))
        .style_func(|row, col| match row {
            TableRow::Header => Style::new().bold(true),
            TableRow::Data(r) if r.is_multiple_of(2) => Style::new().foreground(Color::Green),
            TableRow::Data(_) if col == 1 => Style::new().foreground(Color::Yellow),
            TableRow::Data(_) => Style::new(),
        });
    let text = t.render();
    let mut buf = Buffer::empty(Rect::new(0, 0, 5, 6));
    Widget::render(&t, buf.area, &mut buf);
    assert_eq!(
        buf.content().iter().map(|c| c.symbol()).collect::<String>(),
        "┌─┬─┐│A│B│├─┼─┤│1│2││3│4│└─┴─┘"
    );
    assert_eq!(buf[(0, 0)].fg, Color::Red);
    assert!(buf[(1, 1)].modifier.contains(Modifier::BOLD));
    assert_eq!(buf[(1, 1)].bg, Color::Blue);
    assert_eq!(buf[(1, 3)].fg, Color::Green);
    assert_eq!(buf[(3, 4)].fg, Color::Yellow);
    assert_eq!(buf[(1, 4)].fg, Color::Reset);
    assert_eq!(text.lines.len(), 6);
    assert!(t.render_ansi().contains("\x1b["));
}

#[test]
fn ragged_rows_and_empty_tables() {
    assert_eq!(Table::new().to_string(), "");
    let t = Table::new().row(["a"]).row(["b", "c"]);
    assert_eq!(t.to_string(), "┌─┬─┐\n│a│ │\n│b│c│\n└─┴─┘");
    assert_eq!(t.get_rows().len(), 2);
    assert!(t.get_headers().is_empty());
    assert_eq!(t.clone().clear_rows().to_string(), "");
    assert!(format!("{t:?}").starts_with("Table"));
}

#[test]
fn multiline_cells_grow_rows() {
    let t = Table::new().headers(["k", "v"]).row(["a", "1\n2"]);
    assert_eq!(t.to_string(), "┌─┬─┐\n│k│v│\n├─┼─┤\n│a│1│\n│ │2│\n└─┴─┘");
}

#[test]
fn widget_uses_area_height() {
    let t = languages();
    let mut buf = Buffer::empty(Rect::new(0, 0, 39, 6));
    Widget::render(&t, buf.area, &mut buf);
    let row4: String = (0..39).map(|x| buf[(x, 4)].symbol().to_string()).collect();
    assert!(row4.starts_with("│ …"), "{row4}");
    let row5: String = (0..39).map(|x| buf[(x, 5)].symbol().to_string()).collect();
    assert!(row5.starts_with("└"));
    // an explicit height wins
    let mut buf = Buffer::empty(Rect::new(0, 0, 39, 9));
    Widget::render(&t.clone().height(5), buf.area, &mut buf);
    assert_eq!(to_plain_string(&t.height(5).render()).lines().count(), 5);
}

#[test]
fn markdown_border() {
    let t = Table::new()
        .border(Border::markdown())
        .border_top(false)
        .border_bottom(false)
        .style_func(|_, _| Style::new().padding((0, 1)))
        .headers(["A", "B"])
        .row(["x", "y"]);
    assert_eq!(t.to_string(), "| A | B |\n|---|---|\n| x | y |");
}
