//! A port of Lip Gloss's "layout" demo, made interactive: tabs, a gradient
//! title, a dialog placed in patterned whitespace, lists, a color grid and a
//! status bar, all built from `Style::render` + `join_*` + `place`.
//!
//! ```sh
//! cargo run --example showcase            # interactive (q / Esc to quit)
//! cargo run --example showcase -- --print # print once to stdout
//! cargo run --example showcase -- --light # use the light-background palette
//! ```
//!
//! Keys: `←/→`, `h/l`, `Tab/Shift+Tab`, `1`–`5` switch tabs · `↑/↓`, `j/k`
//! move through the list items · `x`/`Space` toggles an item · `y`/`m` focus
//! a dialog button and `Enter` presses it · `t` toggles light/dark ·
//! `PgUp/PgDn`/wheel scroll · `q`/`Esc`/`Ctrl+C` quit. Everything (tabs,
//! buttons, list items, color swatches) is also clickable.
//!
//! Click handling works by tracking a hit rectangle for every interactive
//! piece while the document is composed (see [`Piece`]), so the rectangles
//! always match exactly what was rendered last.

use std::io;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Color;
use ratatui::text::{Line, Text};
use ratatui::widgets::Paragraph;
use ratatui::{DefaultTerminal, Frame};
use ratatui_lipgloss::{
    AdaptiveColor, Border, Position, Style, Whitespace, blend_1d, color, has_dark_background,
    height, join_horizontal, join_vertical, place_with, set_dark_background, to_ansi_string,
    to_rgb, width,
};

const WIDTH: u16 = 96;
const TABS: [&str; 5] = ["Lip Gloss", "Blush", "Eye Shadow", "Mascara", "Foundation"];
const BUTTONS: [&str; 2] = ["Yes", "Maybe"];
const PRESS_FLASH: Duration = Duration::from_millis(180);

// ---------------------------------------------------------------------------
// Hit tracking
// ---------------------------------------------------------------------------

/// Something that can be clicked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hit {
    Tab(usize),
    Button(usize),
    Item(usize),
    Swatch(Color),
}

/// A rendered block of text plus the hit rectangles inside it (relative to
/// the block's top-left corner). The helpers below mirror the library's
/// join / place / render offsets so rectangles follow their content.
#[derive(Clone, Debug, Default)]
struct Piece {
    text: Text<'static>,
    hits: Vec<(Rect, Hit)>,
}

fn u16_of(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

impl Piece {
    fn new(text: Text<'static>) -> Self {
        Self {
            text,
            hits: Vec::new(),
        }
    }

    /// A piece whose whole area is clickable.
    fn hit(text: Text<'static>, hit: Hit) -> Self {
        let (w, h) = (u16_of(width(&text)), u16_of(height(&text)));
        Self {
            text,
            hits: vec![(Rect::new(0, 0, w, h), hit)],
        }
    }

    fn size(&self) -> (usize, usize) {
        (width(&self.text), height(&self.text))
    }

    fn shifted(hits: Vec<(Rect, Hit)>, dx: usize, dy: usize) -> impl Iterator<Item = (Rect, Hit)> {
        hits.into_iter().map(move |(r, h)| {
            (
                Rect::new(r.x + u16_of(dx), r.y + u16_of(dy), r.width, r.height),
                h,
            )
        })
    }

    /// `style.render(piece)`, keeping hits aligned with the content.
    fn styled(style: &Style, piece: Self) -> Self {
        let (w, h) = piece.size();
        let (p, m, b) = (
            style.get_padding(),
            style.get_margin(),
            style.get_border_sides(),
        );
        let horizontal_padding = usize::from(p.left + p.right);
        let vertical_padding = usize::from(p.top + p.bottom);
        let content_w = w.max(usize::from(style.get_width()).saturating_sub(horizontal_padding));
        let content_h = h.max(usize::from(style.get_height()).saturating_sub(vertical_padding));
        let (ax, _) = style.get_align_horizontal().split(content_w - w);
        let (ay, _) = style.get_align_vertical().split(content_h - h);
        let dx = usize::from(m.left) + usize::from(b.left) + usize::from(p.left) + ax;
        let dy = usize::from(m.top) + usize::from(b.top) + usize::from(p.top) + ay;
        Self {
            text: style.render(piece.text),
            hits: Self::shifted(piece.hits, dx, dy).collect(),
        }
    }
}

fn hjoin(pos: Position, pieces: Vec<Piece>) -> Piece {
    let max_h = pieces.iter().map(|p| p.size().1).max().unwrap_or(0);
    let mut hits = Vec::new();
    let mut x = 0;
    let mut texts = Vec::new();
    for p in pieces {
        let (w, h) = p.size();
        let (dy, _) = pos.split(max_h - h);
        hits.extend(Piece::shifted(p.hits, x, dy));
        x += w;
        texts.push(p.text);
    }
    Piece {
        text: join_horizontal(pos, texts),
        hits,
    }
}

fn vjoin(pos: Position, pieces: Vec<Piece>) -> Piece {
    let max_w = pieces.iter().map(|p| p.size().0).max().unwrap_or(0);
    let mut hits = Vec::new();
    let mut y = 0;
    let mut texts = Vec::new();
    for p in pieces {
        let (w, h) = p.size();
        let (dx, _) = pos.split(max_w - w);
        hits.extend(Piece::shifted(p.hits, dx, y));
        y += h.max(1);
        texts.push(p.text);
    }
    Piece {
        text: join_vertical(pos, texts),
        hits,
    }
}

fn place_piece(w: u16, h: u16, piece: Piece, ws: &Whitespace) -> Piece {
    let (pw, ph) = piece.size();
    let (dx, _) = Position::CENTER.split(usize::from(w).saturating_sub(pw));
    let (dy, _) = Position::CENTER.split(usize::from(h).saturating_sub(ph));
    Piece {
        text: place_with(w, h, Position::CENTER, Position::CENTER, piece.text, ws),
        hits: Piece::shifted(piece.hits, dx, dy).collect(),
    }
}

/// Returns the hit under a screen position.
fn hit_test(hits: &[(Rect, Hit)], column: u16, row: u16) -> Option<Hit> {
    hits.iter()
        .find(|(r, _)| r.contains(ratatui::layout::Position::new(column, row)))
        .map(|(_, h)| *h)
}

/// Converts document-relative hits to screen rectangles for a viewport that
/// shows the document starting at line `scroll` inside `area`.
fn to_screen(hits: &[(Rect, Hit)], area: Rect, scroll: u16) -> Vec<(Rect, Hit)> {
    hits.iter()
        .filter_map(|(r, h)| {
            let offset = i32::from(area.y) - i32::from(scroll);
            let top = (i32::from(r.y) + offset).max(i32::from(area.top()));
            let bottom = (i32::from(r.bottom()) + offset).min(i32::from(area.bottom()));
            let (top, bottom) = (u16::try_from(top).ok()?, u16::try_from(bottom).ok()?);
            if bottom <= top {
                return None;
            }
            let screen = Rect::new(r.x + area.x, top, r.width, bottom - top).intersection(area);
            (!screen.is_empty()).then_some((screen, *h))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Item {
    name: &'static str,
    done: bool,
}

#[derive(Clone, Debug)]
struct App {
    tab: usize,
    button: usize,
    pressed: Option<(usize, Instant)>,
    yes_count: usize,
    answer: Option<usize>,
    items: Vec<Item>,
    selected: usize,
    scroll: u16,
    message: String,
    /// Screen-space hit rectangles from the last render.
    hits: Vec<(Rect, Hit)>,
}

const LIST_SPLIT: usize = 5;

impl App {
    fn new() -> Self {
        let item = |name, done| Item { name, done };
        Self {
            tab: 0,
            button: 0,
            pressed: None,
            yes_count: 0,
            answer: None,
            items: vec![
                item("Grapefruit", true),
                item("Yuzu", true),
                item("Citron", false),
                item("Kumquat", false),
                item("Pomelo", false),
                item("Glossier", false),
                item("Claire's Boutique", false),
                item("Nyx", true),
                item("Mac", false),
                item("Milk", true),
            ],
            selected: 2,
            scroll: 0,
            message: "Ravishing".into(),
            hits: Vec::new(),
        }
    }

    fn select_tab(&mut self, tab: usize) {
        self.tab = tab % TABS.len();
        self.message = format!("Viewing {}", TABS[self.tab]);
    }

    fn press(&mut self, button: usize) {
        self.button = button;
        self.pressed = Some((button, Instant::now()));
        self.answer = Some(button);
        if button == 0 {
            self.yes_count += 1;
            self.message = format!("Marmalade eaten ×{}", self.yes_count);
        } else {
            self.message = "Maybe later…".into();
        }
    }

    fn toggle(&mut self, i: usize) {
        if let Some(item) = self.items.get_mut(i) {
            item.done = !item.done;
            let state = if item.done { "done" } else { "not done" };
            self.message = format!("{} marked {state}", item.name);
        }
    }

    fn select_item(&mut self, i: usize) {
        self.selected = i.min(self.items.len() - 1);
        self.message = format!("Selected {}", self.items[self.selected].name);
    }

    /// Handles a key press; returns `true` to quit.
    fn on_key(&mut self, key: KeyEvent) -> bool {
        let n = self.items.len();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => self.select_tab(self.tab + 1),
            KeyCode::Left | KeyCode::Char('h') | KeyCode::BackTab => {
                self.select_tab(self.tab + TABS.len() - 1);
            }
            KeyCode::Char(c @ '1'..='5') => self.select_tab(usize::from(c as u8 - b'1')),
            KeyCode::Down | KeyCode::Char('j') => self.select_item((self.selected + 1) % n),
            KeyCode::Up | KeyCode::Char('k') => self.select_item((self.selected + n - 1) % n),
            KeyCode::Char('x' | ' ') => self.toggle(self.selected),
            KeyCode::Char('y') => {
                self.button = 0;
                self.message = "Focused “Yes” (Enter to press)".into();
            }
            KeyCode::Char('m') => {
                self.button = 1;
                self.message = "Focused “Maybe” (Enter to press)".into();
            }
            KeyCode::Enter => self.press(self.button),
            KeyCode::Char('t') => {
                set_dark_background(!has_dark_background());
                self.message = if has_dark_background() {
                    "Dark palette".into()
                } else {
                    "Light palette".into()
                };
            }
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(5),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(5),
            _ => {}
        }
        false
    }

    fn on_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                match hit_test(&self.hits, mouse.column, mouse.row) {
                    Some(Hit::Tab(i)) => self.select_tab(i),
                    Some(Hit::Button(b)) => self.press(b),
                    Some(Hit::Item(i)) if i == self.selected => self.toggle(i),
                    Some(Hit::Item(i)) => self.select_item(i),
                    Some(Hit::Swatch(c)) => {
                        let (r, g, b) = to_rgb(c).unwrap_or((0, 0, 0));
                        self.message = format!("Picked #{r:02X}{g:02X}{b:02X}");
                    }
                    None => {}
                }
            }
            MouseEventKind::ScrollDown => self.scroll = self.scroll.saturating_add(2),
            MouseEventKind::ScrollUp => self.scroll = self.scroll.saturating_sub(2),
            _ => {}
        }
    }

    /// Clears expired visual state; returns `true` if a redraw is needed.
    fn tick(&mut self) -> bool {
        if let Some((_, at)) = self.pressed
            && at.elapsed() >= PRESS_FLASH
        {
            self.pressed = None;
            return true;
        }
        false
    }

    fn draw(&mut self, f: &mut Frame) {
        let [doc_area, help_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(f.area());
        let doc = build(self, true);
        let max_scroll = u16_of(height(&doc.text)).saturating_sub(doc_area.height);
        self.scroll = self.scroll.min(max_scroll);
        self.hits = to_screen(&doc.hits, doc_area, self.scroll);
        f.render_widget(Paragraph::new(doc.text).scroll((self.scroll, 0)), doc_area);
        f.render_widget(help_line(), help_area);
    }
}

// ---------------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------------

fn adaptive(light: &str, dark: &str) -> AdaptiveColor {
    AdaptiveColor::new(color(light), color(dark))
}

fn help_line() -> Line<'static> {
    let key = Style::new().foreground("#F25D94").bold(true);
    let text = Style::new().foreground(adaptive("#555555", "#AAAAAA"));
    let mut spans = vec![key.span(" q/Esc"), text.span(" quit")];
    for (k, d) in [
        ("←/→/Tab", "tabs"),
        ("↑/↓/jk", "items"),
        ("x", "toggle"),
        ("y/m/Enter", "dialog"),
        ("t", "theme"),
        ("mouse", "clicks"),
    ] {
        spans.push(text.span(" · "));
        spans.push(key.span(k));
        spans.push(text.span(format!(" {d}")));
    }
    Line::from(spans)
}

fn tabs(app: &App, highlight: AdaptiveColor) -> Piece {
    let active_border = Border {
        top: "─",
        bottom: " ",
        left: "│",
        right: "│",
        top_left: "╭",
        top_right: "╮",
        bottom_left: "┘",
        bottom_right: "└",
        ..Border::empty()
    };
    let tab_border = Border {
        bottom_left: "┴",
        bottom_right: "┴",
        ..Border::rounded()
    };
    let tab = Style::new()
        .border(tab_border)
        .border_foreground(highlight)
        .padding((0, 1));
    let active = Style::new()
        .border(active_border)
        .bold(true)
        .foreground("#F25D94")
        .padding((0, 1))
        .inherit(&tab);

    let row: Vec<Piece> = TABS
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let s = if i == app.tab { &active } else { &tab };
            Piece::hit(s.render(*name), Hit::Tab(i))
        })
        .collect();
    let joined = hjoin(Position::TOP, row);
    let gap_width = usize::from(WIDTH).saturating_sub(joined.size().0);
    let gap = Style::new()
        .border_with_sides(Border::rounded(), (false, false, true, false))
        .border_foreground(highlight)
        .render(" ".repeat(gap_width));
    hjoin(Position::BOTTOM, vec![joined, Piece::new(gap)])
}

fn title(app: &App) -> Piece {
    const GRADIENTS: [(&str, &str); 5] = [
        ("#F25D94", "#EDFF82"),
        ("#FF5F87", "#FFD1DC"),
        ("#643AFF", "#14F9D5"),
        ("#3C3C3C", "#A0A0A0"),
        ("#E0A060", "#FFE4C4"),
    ];
    let (from, to) = GRADIENTS[app.tab];
    let colors = blend_1d(5, &[color(from), color(to)]);
    let bars: Vec<Text<'static>> = colors
        .iter()
        .enumerate()
        .map(|(i, c)| {
            Style::new()
                .margin_left(u16_of(i * 2))
                .padding((0, 1))
                .italic(true)
                .foreground("#FFF7DB")
                .background(*c)
                .render(TABS[app.tab])
        })
        .collect();
    let title = join_vertical(Position::LEFT, bars);
    let muted = Style::new().foreground(adaptive("#A49FA5", "#777777"));
    let desc = join_vertical(
        Position::LEFT,
        [
            Style::new()
                .margin_top(1)
                .render("Style Definitions for Nice Terminal Layouts"),
            muted.render("From Charm · ported to ratatui by ratatui-lipgloss"),
            muted.render(format!("Now viewing: {}", TABS[app.tab])),
        ],
    );
    Piece::new(join_horizontal(
        Position::TOP,
        [title, Style::new().margin_left(3).render(desc)],
    ))
}

fn dialog(app: &App, subtle: AdaptiveColor, interactive: bool) -> Piece {
    let button = Style::new()
        .foreground("#FFF7DB")
        .background("#888B7E")
        .padding((0, 3))
        .margin_top(1)
        .margin_right(2);
    let focused = Style::new()
        .background("#F25D94")
        .underline(true)
        .inherit(&button)
        .padding((0, 3))
        .margin_top(1)
        .margin_right(2);
    let pressed = Style::new()
        .foreground("#F25D94")
        .background("#FFF7DB")
        .bold(true)
        .inherit(&button)
        .padding((0, 3))
        .margin_top(1)
        .margin_right(2);

    let question = match app.answer {
        Some(0) => format!(
            "Delicious! That's {} jar{} of marmalade.",
            app.yes_count,
            if app.yes_count == 1 { "" } else { "s" }
        ),
        Some(_) => "Take your time. The marmalade will wait.".to_string(),
        None => "Are you sure you want to eat marmalade?".to_string(),
    };
    let question = Style::new()
        .width(50)
        .align(Position::CENTER)
        .render(question);
    let buttons: Vec<Piece> = BUTTONS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let style = match app.pressed {
                Some((b, _)) if b == i => &pressed,
                _ if interactive && i == app.button => &focused,
                _ if !interactive && i == 0 => &focused,
                _ => &button,
            };
            // Only the button itself is clickable, not its margins.
            let face = style.clone().unset_margins().render(*label);
            Piece::styled(
                &Style::new().margin_top(1).margin_right(2),
                Piece::hit(face, Hit::Button(i)),
            )
        })
        .collect();
    let ui = vjoin(
        Position::CENTER,
        vec![Piece::new(question), hjoin(Position::TOP, buttons)],
    );
    let boxed = Piece::styled(
        &Style::new()
            .border(Border::rounded())
            .border_foreground("#874BFD")
            .padding((1, 0)),
        ui,
    );
    place_piece(
        WIDTH,
        9,
        boxed,
        &Whitespace::new()
            .chars("猫咪")
            .style(Style::new().foreground(subtle)),
    )
}

fn lists(app: &App, subtle: AdaptiveColor, special: AdaptiveColor, interactive: bool) -> Piece {
    let list = Style::new()
        .border_with_sides(Border::normal(), (false, true, false, false))
        .border_foreground(subtle)
        .margin_right(2)
        .height(8)
        .width(WIDTH / 3);
    let header = Style::new()
        .border_with_sides(Border::normal(), (false, false, true, false))
        .border_foreground(subtle)
        .margin_right(2);
    let check = Style::new().foreground(special).padding_right(1);
    let done = Style::new()
        .strikethrough(true)
        .foreground(adaptive("#969B86", "#696969"));
    let selected = Style::new().foreground("#F25D94").bold(true);

    let item = |i: usize| {
        let it = &app.items[i];
        let is_selected = interactive && i == app.selected;
        let marker = if it.done { "✓" } else { " " };
        let name = match (it.done, is_selected) {
            (_, true) => selected.clone().strikethrough(it.done).render(it.name),
            (true, false) => done.render(it.name),
            (false, false) => Style::new().render(it.name),
        };
        let prefix = if is_selected {
            selected.clone().padding_right(1).render("▸")
        } else {
            check.render(marker)
        };
        let prefix = if is_selected && it.done {
            selected.clone().padding_right(1).render("✓")
        } else {
            prefix
        };
        Piece::hit(join_horizontal(Position::TOP, [prefix, name]), Hit::Item(i))
    };

    let column = |title: &str, range: std::ops::Range<usize>| {
        let mut rows = vec![Piece::new(header.render(title.to_string()))];
        rows.extend(range.map(item));
        vjoin(Position::LEFT, rows)
    };
    let first = column("Citrus Fruits to Try", 0..LIST_SPLIT);
    let second = column("Actual Lip Gloss Vendors", LIST_SPLIT..app.items.len());
    hjoin(
        Position::TOP,
        vec![
            Piece::styled(&list, first),
            Piece::styled(&list.clone().width(WIDTH / 3 - 2), second),
        ],
    )
}

fn color_grid(x_steps: usize, y_steps: usize) -> Piece {
    let (tl, tr) = (color("#F25D94"), color("#EDFF82"));
    let (bl, br) = (color("#643AFF"), color("#14F9D5"));
    let left = blend_1d(y_steps, &[tl, bl]);
    let right = blend_1d(y_steps, &[tr, br]);
    let rows: Vec<Piece> = left
        .iter()
        .zip(&right)
        .map(|(l, r)| {
            let cells: Vec<Piece> = blend_1d(x_steps, &[*l, *r])
                .into_iter()
                .map(|c| Piece::hit(Style::new().background(c).render("  "), Hit::Swatch(c)))
                .collect();
            hjoin(Position::TOP, cells)
        })
        .collect();
    vjoin(Position::LEFT, rows)
}

fn status_bar(app: &App) -> Piece {
    let bar = Style::new()
        .foreground(adaptive("#343433", "#C1C6B2"))
        .background(adaptive("#D9DCCF", "#353533"));
    let status = Style::new()
        .inherit(&bar)
        .foreground("#FFFDF5")
        .background("#FF5F87")
        .padding((0, 1))
        .margin_right(1);
    let encoding = Style::new()
        .inherit(&bar)
        .padding((0, 1))
        .align(Position::RIGHT);
    let fish = Style::new()
        .inherit(&bar)
        .foreground("#FFFDF5")
        .background("#6124DF")
        .padding((0, 1));

    let status_key = status.render("STATUS");
    let encoding = encoding.render("UTF-8");
    let fish = fish.render("🍥 Fish Cake");
    let used = width(&status_key) + width(&encoding) + width(&fish);
    let middle = Style::new()
        .inherit(&bar)
        .width(u16_of(usize::from(WIDTH).saturating_sub(used)))
        .max_height(1)
        .render(app.message.clone());
    Piece::new(join_horizontal(
        Position::TOP,
        [status_key, middle, encoding, fish],
    ))
}

/// Builds the whole document. `interactive` adds focus / selection marks.
fn build(app: &App, interactive: bool) -> Piece {
    let subtle = adaptive("#D9DCCF", "#383838");
    let highlight = adaptive("#874BFD", "#7D56F4");
    let special = adaptive("#43BF6D", "#73F59F");
    let blank = || Piece::new(Text::raw(""));

    let bottom = hjoin(
        Position::TOP,
        vec![lists(app, subtle, special, interactive), color_grid(14, 8)],
    );
    let doc = vjoin(
        Position::LEFT,
        vec![
            tabs(app, highlight),
            blank(),
            title(app),
            blank(),
            dialog(app, subtle, interactive),
            blank(),
            bottom,
            blank(),
            status_bar(app),
        ],
    );
    Piece::styled(&Style::new().padding((1, 2, 1, 2)), doc)
}

// ---------------------------------------------------------------------------
// Terminal setup / main loop
// ---------------------------------------------------------------------------

/// Restores the terminal (mouse capture + raw mode + alternate screen) when
/// dropped, including on early returns.
struct TerminalGuard;

impl TerminalGuard {
    fn init() -> io::Result<(Self, DefaultTerminal)> {
        // `ratatui::init` installs a panic hook that restores the terminal;
        // chain one in front of it that also disables mouse capture.
        let terminal = ratatui::init();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = execute!(io::stdout(), DisableMouseCapture);
            previous(info);
        }));
        execute!(io::stdout(), EnableMouseCapture)?;
        Ok((Self, terminal))
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), DisableMouseCapture);
        ratatui::restore();
    }
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = App::new();
    let mut dirty = true;
    loop {
        if dirty {
            terminal.draw(|f| app.draw(f))?;
            dirty = false;
        }
        let timeout = if app.pressed.is_some() {
            Duration::from_millis(30)
        } else {
            Duration::from_millis(250)
        };
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if app.on_key(key) {
                        return Ok(());
                    }
                    dirty = true;
                }
                Event::Mouse(mouse) => {
                    app.on_mouse(mouse);
                    dirty = true;
                }
                Event::Resize(..) => dirty = true,
                _ => {}
            }
        }
        dirty |= app.tick();
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let dark = !args.iter().any(|a| a == "--light")
        && ratatui_lipgloss::detect_dark_background().unwrap_or(true);
    set_dark_background(dark);

    if args.iter().any(|a| a == "--print") {
        println!("{}", to_ansi_string(&build(&App::new(), false).text));
        return Ok(());
    }

    let (_guard, mut terminal) = TerminalGuard::init()?;
    run(&mut terminal)
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;
    use ratatui::crossterm::event::KeyEventState;
    use ratatui::widgets::Widget;

    use super::*;

    fn render(text: &Text<'_>) -> Buffer {
        let (w, h) = (u16_of(width(text)), u16_of(height(text)));
        let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
        text.clone().render(buf.area, &mut buf);
        buf
    }

    fn text_in(buf: &Buffer, r: Rect) -> String {
        let mut s = String::new();
        for y in r.top()..r.bottom() {
            for x in r.left()..r.right() {
                s.push_str(buf[(x, y)].symbol());
            }
        }
        s
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn click(column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn hit_rects_cover_their_labels() {
        let app = App::new();
        let doc = build(&app, true);
        let buf = render(&doc.text);
        let mut seen = (0, 0, 0, 0);
        for (r, hit) in &doc.hits {
            let s = text_in(&buf, *r);
            match hit {
                Hit::Tab(i) => {
                    assert!(s.contains(TABS[*i]), "tab {i}: {s:?}");
                    seen.0 += 1;
                }
                Hit::Button(i) => {
                    assert_eq!(s.trim(), BUTTONS[*i]);
                    seen.1 += 1;
                }
                Hit::Item(i) => {
                    assert!(s.contains(app.items[*i].name), "item {i}: {s:?}");
                    seen.2 += 1;
                }
                Hit::Swatch(c) => {
                    assert_eq!(buf[(r.x, r.y)].bg, *c);
                    seen.3 += 1;
                }
            }
        }
        assert_eq!(seen, (5, 2, 10, 14 * 8));
    }

    #[test]
    fn screen_mapping_scrolls_and_clips() {
        let hits = vec![
            (Rect::new(2, 1, 4, 3), Hit::Tab(0)),
            (Rect::new(2, 10, 4, 1), Hit::Tab(1)),
        ];
        let area = Rect::new(0, 0, 20, 5);
        let screen = to_screen(&hits, area, 2);
        assert_eq!(screen, vec![(Rect::new(2, 0, 4, 2), Hit::Tab(0))]);
        assert_eq!(hit_test(&screen, 3, 1), Some(Hit::Tab(0)));
        assert_eq!(hit_test(&screen, 3, 2), None);
        assert_eq!(hit_test(&screen, 0, 0), None);
    }

    fn find(app: &App, wanted: Hit) -> (u16, u16) {
        app.hits
            .iter()
            .find(|(_, h)| *h == wanted)
            .map(|(r, _)| (r.x, r.y))
            .expect("hit present")
    }

    fn draw(app: &mut App) {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(110, 60)).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
    }

    #[test]
    fn clicks_drive_the_app() {
        let mut app = App::new();
        draw(&mut app);

        let (x, y) = find(&app, Hit::Tab(3));
        app.on_mouse(click(x, y));
        assert_eq!(app.tab, 3);

        draw(&mut app);
        let (x, y) = find(&app, Hit::Button(1));
        app.on_mouse(click(x + 1, y));
        assert_eq!((app.button, app.answer), (1, Some(1)));
        assert!(app.pressed.is_some());

        draw(&mut app);
        let (x, y) = find(&app, Hit::Item(6));
        app.on_mouse(click(x, y));
        assert_eq!(app.selected, 6);
        assert!(!app.items[6].done);
        app.on_mouse(click(x, y)); // clicking the selected item toggles it
        assert!(app.items[6].done);

        draw(&mut app);
        let (x, y) = app
            .hits
            .iter()
            .find(|(_, h)| matches!(h, Hit::Swatch(_)))
            .map(|(r, _)| (r.x, r.y))
            .unwrap();
        app.on_mouse(click(x, y));
        assert!(app.message.starts_with("Picked #"));
    }

    #[test]
    fn keys_drive_the_app() {
        let mut app = App::new();
        assert!(!app.on_key(key(KeyCode::Right)));
        assert_eq!(app.tab, 1);
        app.on_key(key(KeyCode::BackTab));
        app.on_key(key(KeyCode::Left));
        assert_eq!(app.tab, 4);
        app.on_key(key(KeyCode::Char('2')));
        assert_eq!(app.tab, 1);

        app.on_key(key(KeyCode::Char('j')));
        assert_eq!(app.selected, 3);
        app.on_key(key(KeyCode::Char('x')));
        assert!(app.items[3].done);
        for _ in 0..4 {
            app.on_key(key(KeyCode::Down));
        }
        assert_eq!(app.selected, 7); // moves across both lists

        app.on_key(key(KeyCode::Char('m')));
        assert_eq!(app.button, 1);
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.answer, Some(1));
        app.on_key(key(KeyCode::Char('y')));
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.yes_count, 1);

        assert!(app.on_key(key(KeyCode::Char('q'))));
        assert!(app.on_key(key(KeyCode::Esc)));
    }

    #[test]
    fn help_line_shows_quit_key() {
        let help: String = help_line()
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(help.starts_with(" q/Esc quit"));
        assert!(help.chars().count() <= usize::from(WIDTH));
    }

    #[test]
    fn pressed_state_expires() {
        let mut app = App::new();
        app.press(0);
        assert!(!app.tick());
        app.pressed = Some((0, Instant::now().checked_sub(PRESS_FLASH).unwrap()));
        assert!(app.tick());
        assert!(app.pressed.is_none());
    }
}
