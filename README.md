# ratatui-lipgloss

Declarative, chainable, CSS-like styling for [ratatui](https://ratatui.rs),
inspired by Charm's [Lip Gloss](https://github.com/charmbracelet/lipgloss).

Define a `Style` once (colors, text attributes, padding, margins, borders,
alignment, sizing) and then use it to style ratatui text, wrap ratatui
widgets, or build whole layouts with `join_horizontal`, `join_vertical` and
`place`.

```rust
use ratatui_lipgloss::{Border, Position, Style};

let style = Style::new()
    .bold(true)
    .foreground("#FAFAFA")
    .background("#7D56F4")
    .padding((1, 4))
    .border(Border::rounded())
    .border_foreground("63")
    .width(22)
    .align(Position::CENTER);

// `render` returns a `ratatui::text::Text`, which is a widget:
// frame.render_widget(style.render("Hello, kitty"), area);
let text = style.render("Hello, kitty");
```

## Installation

```sh
cargo add ratatui-lipgloss
```

or in `Cargo.toml`:

```toml
[dependencies]
ratatui = "0.30"
ratatui-lipgloss = "1.0"
```

The crate targets **ratatui 0.30** (MSRV 1.88, same as ratatui). It depends on
`ratatui` with `default-features = false` (plus `std` and `underline-color`),
so it doesn't pick a terminal backend for you.

## Quick start

### 1. Style text, then render it as a widget

```rust
use ratatui::Frame;
use ratatui_lipgloss::{Border, Style};

fn draw(frame: &mut Frame) {
    let card = Style::new()
        .border(Border::rounded())
        .border_foreground("#874BFD")
        .padding((1, 2))
        .width(30);
    frame.render_widget(card.render("Lip Gloss, for ratatui."), frame.area());
}
```

`Style::render` accepts anything that converts into `Text` (`&str`,
`String`, `Span`, `Line`, `Text`). Styles already present on spans are kept
and win over the outer style.

### 2. Wrap any ratatui widget

```rust
use ratatui::widgets::{List, ListState, Paragraph};
use ratatui::Frame;
use ratatui_lipgloss::{Border, Style, WidgetStyleExt};

fn draw(frame: &mut Frame, state: &mut ListState) {
    let panel = Style::new()
        .border(Border::thick())
        .border_foreground("#F25D94")
        .padding((0, 1))
        .margin(1);

    // Widgets get `.styled(&style)`.
    frame.render_widget(Paragraph::new("Hello").styled(&panel), frame.area());

    // StatefulWidgets: use `style.wrap(widget)`.
    let list = List::new(["one", "two"]).highlight_symbol("> ");
    frame.render_stateful_widget(panel.wrap(list), frame.area(), state);
}
```

The wrapper draws margins, border and padding and applies the style's colors
and attributes to the box, then renders the widget in the remaining area.
`Style::layout(area)` / `Style::inner(area)` give you the rectangles if you
do the rendering yourself.

### 3. Use it as a plain ratatui style

`&Style` converts into `ratatui::style::Style`, so it works anywhere ratatui
takes a style:

```rust
use ratatui::text::Span;
use ratatui_lipgloss::Style;

let warn = Style::new().foreground("#FFCC00").bold(true);
let span = Span::styled("careful", &warn);
let patched = warn.apply(Span::raw("keeps its own styles too"));
```

### 4. Build layouts

```rust
use ratatui_lipgloss::{join_horizontal, join_vertical, place, Border, Position, Style};

let a = Style::new().border(Border::normal()).render("left");
let b = Style::new().border(Border::double()).padding((1, 2)).render("right");
let row = join_horizontal(Position::CENTER, [a, b]);
let page = join_vertical(Position::LEFT, [row, "footer".into()]);
let centered = place(80, 24, Position::CENTER, Position::CENTER, page);
```

### 5. Print outside a TUI

```rust
use ratatui_lipgloss::Style;

println!("{}", Style::new().italic(true).foreground("201").render_ansi("Hi!"));
```

## Examples

```sh
cargo run --example quickstart           # README example, printed with ANSI codes
cargo run --example borders              # every border preset, per-side borders, custom patterns
cargo run --example widgets              # Paragraph / List / Gauge wrapped in styles (TUI, keys + mouse, q/Esc quits)
cargo run --example showcase             # interactive port of Lip Gloss's layout demo (TUI, keys + mouse, q/Esc quits)
cargo run --example showcase -- --print  # same, printed once to stdout
cargo run --example showcase -- --light  # light-background adaptive palette
```

The showcase is fully interactive. Use `←/→`, `h/l`, `Tab/Shift+Tab` or
`1`–`5` to switch tabs. Use `↑/↓` or `j/k` to move through the list items and
`x` or `Space` to toggle one. Use `y`/`m` to focus a dialog button and
`Enter` to press it. `t` toggles the light/dark palette, `PgUp/PgDn` and the
mouse wheel scroll, and `q`/`Esc` quits. Tabs, buttons, list items and color
swatches are also clickable. Click handling uses hit rectangles that are
tracked while the layout is composed, so they always match the last frame.

## Concepts

### Colors

Every color rule takes `impl Into<TerminalColor>`:

| You write | Meaning |
|---|---|
| `"#7D56F4"`, `"#fff"` | true color |
| `"86"` / `86u8` | ANSI 256 index |
| `"magenta"`, `"light-blue"` | named ANSI color |
| `(125, 86, 244)` | true color |
| `Color::Red` | any ratatui `Color` |
| `AdaptiveColor::new(light, dark)` | picked at render time |

Adaptive colors resolve against a global flag (`set_dark_background`,
default dark). `detect_dark_background()` reads `COLORFGBG` without touching
stdin. The `render_with(content, dark)` and `to_ratatui_with(dark)` variants
take the flag explicitly. Utilities: `blend`, `blend_1d`, `darken`, `lighten`,
`complementary`, `to_rgb`.

### Box model

`render` does the following, in order:

1. Converts tabs (4 spaces by default, see `tab_width`).
2. Word-wraps to `width - horizontal padding`.
3. Aligns lines horizontally within the width.
4. Pads vertically to `height - vertical padding` using `align_vertical`.
5. Adds padding (background-colored unless `color_whitespace(false)`).
6. Adds the border.
7. Adds margins (colored with `margin_background`).
8. Truncates to `max_width` / `max_height`.

As in Lip Gloss, **`width` and `height` include padding but not borders or
margins**. `inline(true)` puts everything on one line and skips padding,
borders and margins.

### Rules are optional

Every rule can be *unset* (`unset_bold()`, `unset_padding()`, ...).
`a.inherit(&b)` copies the rules `b` sets that `a` leaves unset. Like Lip
Gloss, margins, padding and the string value are not inherited. `Style` is a
plain value, so `clone()` makes a true copy.

## Feature comparison with Lip Gloss

| Lip Gloss | ratatui-lipgloss | Status |
|---|---|---|
| `NewStyle()`, value semantics / copy | `Style::new()`, `Clone` | ✅ |
| `Foreground`, `Background` | `foreground`, `background` | ✅ |
| `Color("#hex" / "86" / "5")`, named ANSI | `&str` / `u8` / `(r,g,b)` / ratatui `Color` | ✅ |
| `AdaptiveColor`, `LightDark`, `HasDarkBackground` | `AdaptiveColor`, `light_dark`, `set_dark_background`, `detect_dark_background` (env-based) | ✅ (no terminal query) |
| `CompleteColor` / `Complete(profile)` | — | ❌ (ratatui backends handle color output) |
| Automatic color downsampling | — | ❌ (up to the backend / terminal) |
| `Darken`, `Lighten`, `Complementary`, `Blend1D` | `darken`, `lighten`, `complementary`, `blend`, `blend_1d` | ✅ (RGB blending, not CIELAB) |
| `Alpha`, `Blend2D` | — | ❌ |
| `Bold`, `Italic`, `Underline`, `Strikethrough`, `Faint`, `Reverse`, `Blink` | same names (+ `hidden`) | ✅ |
| `UnderlineColor` | `underline_color` | ✅ |
| `UnderlineStyle` (curly, dotted, ...) | — | ❌ (not in ratatui's style model) |
| `Hyperlink` | — | ❌ |
| `Padding*`, `Margin*`, CSS shorthand | `padding*`, `margin*`, `(v, h)` / `(t, h, b)` / `(t, r, b, l)` | ✅ |
| `PaddingChar`, `MarginChar`, `MarginBackground` | `padding_char`, `margin_char`, `margin_background` | ✅ |
| `ColorWhitespace` | `color_whitespace` | ✅ |
| `Width`, `Height`, `MaxWidth`, `MaxHeight` | same | ✅ |
| `Align`, `AlignHorizontal`, `AlignVertical` (fractional positions) | `align`, `align_horizontal`, `align_vertical` with `Position` / `f32` / ratatui `Alignment` | ✅ |
| Border presets: normal, rounded, thick, double, block, outer/inner half block, hidden, ASCII, Markdown | `Border::normal()` … `Border::markdown()` | ✅ |
| Custom borders with repeating patterns | `Border { .. }` (`&'static str` fields) | ✅ |
| `Border(b, sides...)`, `BorderTop/Right/Bottom/Left` | `border`, `border_with_sides`, `border_top` … | ✅ |
| Per-side `BorderForeground` / `BorderBackground` | `border_*_foreground`, `border_*_background` | ✅ |
| `BorderForegroundBlend` (gradient borders) | — | ❌ |
| `Inherit`, `Unset*`, `Get*` | `inherit`, `unset_*`, `get_*` | ✅ |
| `Inline` | `inline` | ✅ |
| `TabWidth` | `tab_width` (0 removes tabs) | ✅ (no `NoTabConversion`) |
| `SetString`, `Stringer` | `set_string`, `Display` | ✅ |
| `Transform` | — | ❌ |
| `Render` → string | `render` → `Text`, `render_ansi` → `String`, `render_plain` | ✅ |
| `Width`, `Height`, `Size` (measuring) | `width`, `height`, `size` on `Text` | ✅ |
| `JoinHorizontal`, `JoinVertical` | `join_horizontal`, `join_vertical` | ✅ |
| `Place`, `PlaceHorizontal`, `PlaceVertical`, whitespace options | `place`, `place_horizontal`, `place_vertical`, `place_with` + `Whitespace` | ✅ |
| `Wrap` | built into `width` | ⚠️ (no standalone function) |
| Compositor / layers | — | ❌ (use ratatui's layout + buffer) |
| `table`, `list`, `tree` sub-packages | — | ❌ (planned; use ratatui's `Table` / `List` wrapped with `.styled()`) |
| *(ratatui only)* wrap widgets & stateful widgets | `StyledWidget`, `WidgetStyleExt::styled`, `Style::wrap` | ✅ |
| *(ratatui only)* to/from `ratatui::style::Style`, `Block` | `From` impls, `Style::apply`, `Style::to_block` | ✅ |

## Development

```sh
cargo test
cargo clippy --all-targets
cargo fmt --check
cargo doc --no-deps --open
```

## License

Licensed under the BSD 3-Clause License. See [LICENSE](LICENSE) for details.

Lip Gloss is © Charmbracelet, Inc. and MIT-licensed. This crate is an
independent Rust re-implementation of its API ideas and is not affiliated
with Charm.
