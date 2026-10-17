//! The Lip Gloss README example, printed straight to the terminal (no TUI).
//!
//! ```sh
//! cargo run --example quickstart
//! ```

use ratatui_lipgloss::{Border, Position, Style};

fn main() {
    // lipgloss.NewStyle().Bold(true).Foreground(...).Background(...)
    //     .PaddingTop(2).PaddingLeft(4).Width(22)
    let style = Style::new()
        .bold(true)
        .foreground("#FAFAFA")
        .background("#7D56F4")
        .padding_top(2)
        .padding_left(4)
        .width(22);
    println!("{}\n", style.render_ansi("Hello, kitty"));

    // Borders, alignment, inheritance.
    let base = Style::new()
        .foreground("#F25D94")
        .border(Border::rounded())
        .border_foreground("63")
        .padding((0, 2));
    let centered = Style::new()
        .width(30)
        .align(Position::CENTER)
        .italic(true)
        .padding((0, 2)) // padding & margins are never inherited
        .inherit(&base);
    println!(
        "{}\n",
        centered.render_ansi("Lip Gloss, for ratatui.\nCentered and wrapped to fit.")
    );

    // A style used inline, as a plain ratatui style.
    let warning = Style::new()
        .foreground("#FFCC00")
        .bold(true)
        .underline(true);
    let span = warning.span("This span was styled with ratatui-lipgloss");
    println!(
        "{}",
        ratatui_lipgloss::to_ansi_string(&ratatui::text::Text::from(span))
    );

    // Styles also implement Display via set_string, like Lip Gloss's Stringer.
    println!(
        "{}",
        Style::new()
            .set_string("你好，猫咪。")
            .bold(true)
            .foreground("#04B575")
    );
}
