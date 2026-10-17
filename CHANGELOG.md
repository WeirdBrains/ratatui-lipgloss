# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.1.0] - 2026-10-17

### Added

- `table` module, a port of Lip Gloss's `table` package. `Table` has
  headers, rows and a per-cell `StyleFunc(TableRow, column)` that inherits
  a `base_style`. It supports any `Border` preset with toggles for the
  outer edges and the header, column and row separators, and a
  `border_style`. A fixed `width` grows or shrinks columns, and long cells
  wrap or are truncated (`wrap(false)`). A fixed `height` shows an `…`
  overflow row, `y_offset` scrolls, and `visible_rows` reports the rows
  shown. It renders to `Text`, `render_ansi` or `Display`, and `&Table`
  is a ratatui `Widget`.
- `list` module, a port of Lip Gloss's `list` package. `List` comes with
  bullet, dash, asterisk, arabic, alphabet and roman enumerators or a
  custom function. It supports nested lists and trees, enumerator, indenter
  and item styles (fixed or per-item functions), `hide` and `offset`. It
  renders to `Text`, and `&List` is a `Widget` and a `StatefulWidget`.
- `tree` module, a port of Lip Gloss's `tree` package. `Tree`, `Leaf` and
  `Node` support the default (`├──`/`└──`) and rounded (`╰──`) enumerators,
  custom enumerators and indenters, and root, item and enumerator styles.
  Nodes can be hidden, collapsed or cut with `offset`, multi-line items
  and nested renderers work, and `width` sets a fixed width. It renders to
  `Text`.
- `TreeState` and `StatefulWidget for &Tree` make interactive trees
  possible:
  - Selection by path (`select_next`/`previous`/`first`/`last`).
  - `collapse`/`expand`/`toggle`.
  - Automatic scrolling to the selection.
  - A `highlight_style` for the selected row.
  - `path_at(column, row)` for mouse hit testing.
- Gradient borders. `Style::border_foreground_blend` blends two or more
  colors clockwise around the border perimeter, as in Lip Gloss v2.
  `Style::border_foreground_blend_offset` rotates the gradient. They come
  with matching `get_*`/`unset_*` methods and work in `render`,
  `render_ansi` and `StyledWidget`.
- New examples: `table`, `list` and `tree` (interactive, with an animated
  gradient border). The `borders` example gains gradient borders.
- `List`, `Table`, `TableRow`, `Tree` and `TreeState` are in the prelude.

## [1.0.0] - 2026-10-17

Initial release.

### Added

- `Style` builder modeled on Lip Gloss: colors (string / ANSI 256 / true color /
  adaptive), text attributes (bold, italic, underline, strikethrough, faint,
  reverse, blink, hidden, underline color), padding, margins, width/height,
  max sizes, horizontal & vertical alignment with fractional `Position`s,
  borders with per-side toggles and colors, `inherit`, `unset_*` / `get_*`,
  `set_string` + `Display`, inline mode, tab width, padding/margin fill
  characters, margin background and `color_whitespace`.
- `Style::render` → `ratatui::text::Text`, plus `render_with`, `render_ansi`,
  `render_plain`, `to_ansi_string` and `to_plain_string`.
- `StyledWidget` / `WidgetStyleExt::styled` / `Style::wrap` to render any
  `Widget` or `StatefulWidget` inside a style's box model;
  `Style::layout` / `Style::inner` / `Style::render_frame` and `BoxLayout`.
- Border presets: normal, rounded, thick, double, block, outer/inner half
  block, hidden, ASCII, Markdown; custom borders with repeating patterns;
  conversions to/from ratatui `border::Set`.
- Layout helpers: `join_horizontal`, `join_vertical`, `place`,
  `place_horizontal`, `place_vertical`, `place_with` + `Whitespace`,
  `width` / `height` / `size`.
- Color utilities: `parse_color`, `color`, `TerminalColor`, `AdaptiveColor`,
  `light_dark`, `set_dark_background` / `has_dark_background`,
  `detect_dark_background`, `blend`, `blend_1d`, `darken`, `lighten`,
  `complementary`, `to_rgb`.
- Conversions to/from `ratatui::style::Style`, `Style::apply`, `Style::span`,
  `Style::line`, `Style::to_block`; `Sides` / `BorderSides` CSS shorthands.
- Examples: `quickstart`, `borders`, `widgets` (wrapped Paragraph / List /
  Gauge with keyboard and mouse selection) and an interactive `showcase`
  port of Lip Gloss's layout demo: clickable and keyboard-driven tabs,
  dialog buttons with focus / pressed states, selectable and toggleable list
  items, clickable color swatches, theme toggle, scrolling, an always-visible
  key help line, and `--print` / `--light` modes.
- Licensed under the BSD 3-Clause License.
