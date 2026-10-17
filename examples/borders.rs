//! Every border preset, plus per-side toggles and colors and gradient
//! (blended) borders, printed with `join_horizontal` / `join_vertical`.
//!
//! ```sh
//! cargo run --example borders
//! ```

use ratatui::text::Text;
use ratatui_lipgloss::{Border, Position, Style, join_horizontal, join_vertical, to_ansi_string};

fn boxed(name: &str, border: Border, color: &str) -> Text<'static> {
    Style::new()
        .border(border)
        .border_foreground(color)
        .padding((0, 1))
        .width(18)
        .align(Position::CENTER)
        .margin_right(1)
        .render(name.to_string())
}

fn main() {
    let row1 = join_horizontal(
        Position::TOP,
        [
            boxed("normal", Border::normal(), "#7D56F4"),
            boxed("rounded", Border::rounded(), "#F25D94"),
            boxed("thick", Border::thick(), "#04B575"),
            boxed("double", Border::double(), "#EE6FF8"),
        ],
    );
    let row2 = join_horizontal(
        Position::TOP,
        [
            boxed("block", Border::block(), "#3C3C3C"),
            boxed("outer half block", Border::outer_half_block(), "#FF5F87"),
            boxed("inner half block", Border::inner_half_block(), "#5FAFFF"),
            boxed("hidden", Border::hidden(), "#FFFFFF"),
        ],
    );
    let row3 = join_horizontal(
        Position::TOP,
        [
            boxed("ascii", Border::ascii(), "#AAAAAA"),
            boxed("markdown", Border::markdown(), "#AAAAAA"),
            Style::new()
                .border_with_sides(Border::double(), (true, false, false, true))
                .border_top_foreground("#FF0000")
                .border_left_foreground("#00FF00")
                .padding((0, 1))
                .margin_right(1)
                .render("top + left only"),
            Style::new()
                .border(Border {
                    top: "._.:*:",
                    bottom: "._.:*:",
                    left: "|*",
                    right: "|*",
                    top_left: "*",
                    top_right: "*",
                    bottom_left: "*",
                    bottom_right: "*",
                    ..Border::empty()
                })
                .border_foreground("#FFD700")
                .padding((0, 1))
                .render("custom\npattern"),
        ],
    );
    // Gradient borders blend their colors clockwise around the perimeter,
    // starting at the top-left corner; the offset rotates the gradient.
    let gradient = |name: &str, border: Border, colors: &[&str], offset: i32| {
        Style::new()
            .border(border)
            .border_foreground_blend(colors.iter().copied())
            .border_foreground_blend_offset(offset)
            .padding((1, 2))
            .width(18)
            .align(Position::CENTER)
            .margin_right(1)
            .render(name.to_string())
    };
    let row4 = join_horizontal(
        Position::TOP,
        [
            gradient("two stops", Border::rounded(), &["#F25D94", "#874BFD"], 0),
            gradient(
                "rainbow",
                Border::thick(),
                &[
                    "#FF0000", "#FFFF00", "#00FF00", "#00FFFF", "#0000FF", "#FF00FF",
                ],
                0,
            ),
            gradient("offset 10", Border::rounded(), &["#F25D94", "#874BFD"], 10),
            gradient(
                "double",
                Border::double(),
                &["#04B575", "#FFFDF5", "#04B575"],
                0,
            ),
        ],
    );
    println!(
        "{}",
        to_ansi_string(&join_vertical(Position::LEFT, [row1, row2, row3, row4]))
    );
}
