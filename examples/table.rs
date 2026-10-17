//! Tables: headers, per-cell styles, border presets and separators, fixed
//! width and height with scrolling.
//!
//! ```sh
//! cargo run --example table
//! ```

use ratatui::text::Text;
use ratatui_lipgloss::table::{Table, TableRow};
use ratatui_lipgloss::{Border, Position, Style, join_vertical, to_ansi_string};

const PURPLE: &str = "#874BFD";
const GRAY: &str = "#A0A0A0";
const LIGHT_GRAY: &str = "#D0D0D0";

fn languages() -> Table {
    Table::new()
        .headers(["LANGUAGE", "FORMAL", "INFORMAL"])
        .row(["Chinese", "Nǐn hǎo", "Nǐ hǎo"])
        .row(["French", "Bonjour", "Salut"])
        .row(["Japanese", "こんにちは", "やあ"])
        .row(["Russian", "Zdravstvuyte", "Privet"])
        .row(["Spanish", "Hola", "¿Qué tal?"])
        .row(["Arabic", "أهلين", "أهلا"])
}

fn caption(s: &str) -> Text<'static> {
    Style::new()
        .foreground(PURPLE)
        .bold(true)
        .margin_top(1)
        .render(s.to_string())
}

fn main() {
    // Lip Gloss's classic table example: header, zebra rows, one column
    // highlighted.
    let striped = languages()
        .border(Border::thick())
        .border_style(Style::new().foreground(PURPLE))
        .style_func(|row, col| {
            let cell = Style::new().padding((0, 1));
            match row {
                TableRow::Header => cell.foreground(PURPLE).bold(true).align(Position::CENTER),
                TableRow::Data(_) if col == 1 => cell.foreground("#F25D94"),
                TableRow::Data(r) if r.is_multiple_of(2) => cell.foreground(GRAY),
                TableRow::Data(_) => cell.foreground(LIGHT_GRAY),
            }
        });

    // Row separators, rounded corners and a fixed width of 50 columns.
    let wide = languages()
        .border(Border::rounded())
        .border_row(true)
        .border_style(Style::new().foreground("#04B575"))
        .style_func(|_, _| Style::new().padding((0, 1)))
        .width(50);

    // Only inner borders, like a spreadsheet.
    let inner = languages()
        .border_top(false)
        .border_bottom(false)
        .border_left(false)
        .border_right(false)
        .style_func(|_, _| Style::new().padding((0, 1)));

    // A fixed height scrolls: rows 2.. are shown and the rest is elided.
    let scrolled = languages()
        .border(Border::double())
        .style_func(|_, _| Style::new().padding((0, 1)))
        .height(7)
        .y_offset(2);

    // Markdown output.
    let markdown = languages()
        .border(Border::markdown())
        .border_top(false)
        .border_bottom(false)
        .style_func(|_, _| Style::new().padding((0, 1)));

    let out = join_vertical(
        Position::LEFT,
        [
            caption("Styled cells"),
            striped.render(),
            caption("Row separators, width 50"),
            wide.render(),
            caption("Inner borders only"),
            inner.render(),
            caption(&format!(
                "Height 7, y_offset 2 (rows {:?} visible)",
                scrolled.visible_rows()
            )),
            scrolled.render(),
            caption("Markdown"),
            markdown.render(),
        ],
    );
    println!("{}", to_ansi_string(&out));
}
