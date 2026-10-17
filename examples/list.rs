//! Lists: every built-in enumerator, nesting, a custom enumerator and
//! per-item styles.
//!
//! ```sh
//! cargo run --example list
//! ```

use ratatui_lipgloss::list::{self, Items, List};
use ratatui_lipgloss::{Position, Style, join_horizontal, to_ansi_string};

const PINK: &str = "#F25D94";
const PURPLE: &str = "#874BFD";
const GREEN: &str = "#04B575";

fn column(title: &str, l: List) -> ratatui::text::Text<'static> {
    let title = Style::new()
        .foreground(PURPLE)
        .bold(true)
        .render(title.to_string());
    Style::new()
        .margin_right(4)
        .render(ratatui_lipgloss::join_vertical(
            Position::LEFT,
            [title, l.render()],
        ))
}

fn checkbox(_: &Items<'_>, i: usize) -> String {
    if i.is_multiple_of(2) {
        "✓".into()
    } else {
        "•".into()
    }
}

fn main() {
    let fruit = || ["Apple", "Banana", "Cherry", "Durian"];
    let enumerators = join_horizontal(
        Position::TOP,
        [
            column("bullet", List::new(fruit())),
            column("dash", List::new(fruit()).enumerator(list::dash)),
            column("asterisk", List::new(fruit()).enumerator(list::asterisk)),
            column("arabic", List::new(fruit()).enumerator(list::arabic)),
            column("alphabet", List::new(fruit()).enumerator(list::alphabet)),
            column("roman", List::new(fruit()).enumerator(list::roman)),
        ],
    );

    let groceries = List::new(["Produce"])
        .item(
            List::new(["Apples", "Oranges"])
                .item(List::new(["Lemons", "Limes"]).enumerator(list::dash))
                .enumerator(list::roman),
        )
        .item("Dairy")
        .item(List::new(["Milk", "Yogurt"]).enumerator(list::alphabet))
        .item("Bakery")
        .enumerator(list::arabic)
        .enumerator_style(Style::new().foreground(PINK).padding_right(1))
        .item_style(Style::new().foreground("#DDDDDD"));

    let done = List::new(["Buy gloss", "Find mirror", "Apply", "Admire"])
        .enumerator(checkbox)
        .enumerator_style_func(|_, i| {
            Style::new()
                .foreground(if i.is_multiple_of(2) {
                    GREEN
                } else {
                    "#666666"
                })
                .padding_right(1)
        })
        .item_style_func(|_, i| {
            if i.is_multiple_of(2) {
                Style::new().strikethrough(true).foreground("#666666")
            } else {
                Style::new()
            }
        });

    let out = ratatui_lipgloss::join_vertical(
        Position::LEFT,
        [
            enumerators,
            Style::new().render(""),
            join_horizontal(
                Position::TOP,
                [column("nested", groceries), column("custom", done)],
            ),
        ],
    );
    println!("{}", to_ansi_string(&out));
}
