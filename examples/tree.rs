//! An interactive tree: a file browser built from `tree::Tree` and
//! `tree::TreeState`, inside a panel with an animated gradient border.
//!
//! Keys: `↑/↓` or `j/k` move, `Enter`/`Space` toggles a directory, `←/h`
//! collapses, `→/l` expands, `g`/`G` jump to the first/last node; click a
//! node to select it (click again to toggle), scroll to move; `q`/`Esc`
//! quits. `--print` prints the tree once without entering the TUI.
//!
//! ```sh
//! cargo run --example tree
//! cargo run --example tree -- --print
//! ```

use std::io;
use std::time::Duration;

use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseButton, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Constraint, Layout};
use ratatui::text::Line;
use ratatui::{DefaultTerminal, Frame};
use ratatui_lipgloss::tree::{self, NodeRef, Tree, TreeState};
use ratatui_lipgloss::{Border, Style, to_ansi_string};

const PINK: &str = "#F25D94";
const PURPLE: &str = "#874BFD";

fn dir(name: &str) -> Tree {
    tree::root(format!("{name}/"))
}

fn project() -> Tree {
    dir("ratatui-lipgloss")
        .child(
            dir("src")
                .children([
                    "lib.rs",
                    "style.rs",
                    "border.rs",
                    "color.rs",
                    "widget.rs",
                    "table.rs",
                    "list.rs",
                ])
                .child("tree.rs"),
        )
        .child(dir("examples").children([
            "quickstart.rs",
            "borders.rs",
            "table.rs",
            "list.rs",
            "tree.rs",
            "widgets.rs",
            "showcase.rs",
        ]))
        .child(dir("tests").children([
            "render.rs",
            "widget.rs",
            "gradient.rs",
            "table.rs",
            "tree.rs",
        ]))
        .child("Cargo.toml")
        .child("CHANGELOG.md")
        .child("LICENSE")
        .child("README.md")
        .enumerator(tree::rounded_enumerator)
        .enumerator_style(Style::new().foreground(PURPLE).padding_right(1))
        .indenter_style(Style::new().foreground(PURPLE).padding_right(1))
        .root_style(Style::new().foreground(PINK).bold(true))
        .item_style_func(|items, i| {
            if items.get(i).is_some_and(|n| !n.children().is_empty()) {
                Style::new().foreground("#5FAFFF").bold(true)
            } else {
                Style::new()
            }
        })
        .highlight_style(Style::new().background("#3C3C3C").foreground(PINK))
}

struct App {
    tree: Tree,
    state: TreeState,
    /// Rotation of the border gradient, advanced on every tick.
    tick: i32,
}

impl App {
    fn new() -> Self {
        let tree = project();
        let mut state = TreeState::new();
        state.select_first(&tree);
        Self {
            tree,
            state,
            tick: 0,
        }
    }

    fn is_dir(&self, path: &[usize]) -> bool {
        matches!(self.tree.node_at(path), Some(NodeRef::Tree(t)) if !t.nodes().is_empty())
    }

    fn draw(&mut self, f: &mut Frame) {
        let [body, help] =
            Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(f.area());
        let panel = Style::new()
            .border(Border::rounded())
            .border_foreground_blend([PINK, PURPLE, "#5FAFFF", PINK])
            .border_foreground_blend_offset(self.tick)
            .padding((0, 1));
        // StyledWidget works with stateful widgets too; the tree records its
        // inner area in the state for mouse hit tests.
        f.render_stateful_widget(panel.wrap(&self.tree), body, &mut self.state);

        let key = Style::new().foreground(PINK).bold(true);
        let muted = Style::new().foreground("#888888");
        f.render_widget(
            Line::from(vec![
                key.span(" q/Esc"),
                muted.span(" quit · "),
                key.span("↑/↓"),
                muted.span(" move · "),
                key.span("enter"),
                muted.span(" toggle · "),
                key.span("←/→"),
                muted.span(" collapse/expand · "),
                key.span("click"),
                muted.span(" select"),
            ]),
            help,
        );
    }

    /// Handles a key; returns `false` to quit.
    fn key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return false,
            KeyCode::Down | KeyCode::Char('j') => self.state.select_next(&self.tree),
            KeyCode::Up | KeyCode::Char('k') => self.state.select_previous(&self.tree),
            KeyCode::Char('g') | KeyCode::Home => self.state.select_first(&self.tree),
            KeyCode::Char('G') | KeyCode::End => self.state.select_last(&self.tree),
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(p) = self.state.selected().map(<[usize]>::to_vec)
                    && self.is_dir(&p)
                {
                    self.state.toggle(p);
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(p) = self.state.selected().map(<[usize]>::to_vec) {
                    if self.is_dir(&p) && !self.state.is_collapsed(&p) {
                        self.state.collapse(p);
                    } else if !p.is_empty() {
                        // jump to the parent
                        self.state.select(Some(p[..p.len() - 1].to_vec()));
                    }
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if let Some(p) = self.state.selected().map(<[usize]>::to_vec) {
                    self.state.expand(&p);
                }
            }
            _ => {}
        }
        true
    }

    fn click(&mut self, column: u16, row: u16) {
        if let Some(p) = self.state.path_at(column, row) {
            if self.state.selected() == Some(&p[..]) && self.is_dir(&p) {
                self.state.toggle(p);
            } else {
                self.state.select(Some(p));
            }
        }
    }
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = App::new();
    loop {
        terminal.draw(|f| app.draw(f))?;
        if !event::poll(Duration::from_millis(80))? {
            app.tick += 1;
            continue;
        }
        match event::read()? {
            Event::Key(k) if k.kind == KeyEventKind::Press => {
                if !app.key(k.code, k.modifiers) {
                    return Ok(());
                }
            }
            Event::Mouse(m) => match m.kind {
                MouseEventKind::Down(MouseButton::Left) => app.click(m.column, m.row),
                MouseEventKind::ScrollDown => app.state.select_next(&app.tree),
                MouseEventKind::ScrollUp => app.state.select_previous(&app.tree),
                _ => {}
            },
            _ => {}
        }
    }
}

fn main() -> io::Result<()> {
    if std::env::args().any(|a| a == "--print") {
        println!("{}", to_ansi_string(&project().render()));
        return Ok(());
    }
    let mut terminal = ratatui::init();
    // ratatui::init's panic hook restores the terminal; also drop mouse capture.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), DisableMouseCapture);
        previous(info);
    }));
    execute!(io::stdout(), EnableMouseCapture)?;
    let result = run(&mut terminal);
    let _ = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(app: &mut App) -> ratatui::Terminal<ratatui::backend::TestBackend> {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 12)).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
        terminal
    }

    fn line(t: &ratatui::Terminal<ratatui::backend::TestBackend>, y: u16) -> String {
        let buf = t.backend().buffer();
        (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect()
    }

    #[test]
    fn keys_navigate_and_toggle() {
        let mut app = App::new();
        assert!(app.key(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.state.selected(), Some(&[0][..]));
        app.key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.state.is_collapsed(&[0]));
        app.key(KeyCode::Right, KeyModifiers::NONE);
        assert!(!app.state.is_collapsed(&[0]));
        app.key(KeyCode::Down, KeyModifiers::NONE);
        app.key(KeyCode::Left, KeyModifiers::NONE); // leaf: go to parent
        assert_eq!(app.state.selected(), Some(&[0][..]));
        app.key(KeyCode::Char('G'), KeyModifiers::NONE);
        assert_eq!(app.state.selected(), Some(&[6][..]));
        app.key(KeyCode::Enter, KeyModifiers::NONE); // a file: no-op
        assert!(!app.state.is_collapsed(&[6]));
        assert!(!app.key(KeyCode::Char('q'), KeyModifiers::NONE));
    }

    #[test]
    fn draws_tree_help_and_handles_clicks() {
        let mut app = App::new();
        let t = draw(&mut app);
        assert!(
            line(&t, 1).contains("ratatui-lipgloss/"),
            "{:?}",
            line(&t, 1)
        );
        assert!(line(&t, 2).contains("├── src/"), "{:?}", line(&t, 2));
        assert!(line(&t, 11).starts_with(" q/Esc quit"));
        // click "src/" twice: select, then collapse
        let col = u16::try_from(line(&t, 2).chars().position(|c| c == 's').unwrap()).unwrap();
        app.click(col, 2);
        assert_eq!(app.state.selected(), Some(&[0][..]));
        app.click(col, 2);
        assert!(app.state.is_collapsed(&[0]));
        let t = draw(&mut app);
        assert!(line(&t, 3).contains("examples/"), "{:?}", line(&t, 3));
        app.click(0, 0); // on the border: nothing
        assert_eq!(app.state.selected(), Some(&[0][..]));
    }
}
