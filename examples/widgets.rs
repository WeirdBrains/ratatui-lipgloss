//! Wrapping regular ratatui widgets (stateless and stateful) with
//! ratatui-lipgloss styles.
//!
//! Keys: `↑/↓` or `j/k` move the selection, click a list item to select it,
//! `q` / `Esc` quits.
//!
//! ```sh
//! cargo run --example widgets
//! ```

use std::io;

use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseButton, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Gauge, List, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use ratatui_lipgloss::{AdaptiveColor, Border, Position, Style, WidgetStyleExt};

const ITEMS: [&str; 6] = [
    "Glossier",
    "Claire's Boutique",
    "Nyx",
    "Mac",
    "Milk",
    "Fenty",
];

struct Styles {
    panel: Style,
    focused: Style,
    title: Style,
    gauge: Style,
}

impl Styles {
    fn new() -> Self {
        let panel = Style::new()
            .border(Border::rounded())
            .border_foreground(AdaptiveColor::new(
                ratatui_lipgloss::color("#C5ADF9"),
                ratatui_lipgloss::color("#874BFD"),
            ))
            .padding((0, 1))
            .margin((0, 1));
        Self {
            focused: Style::new()
                .border(Border::thick())
                .border_foreground("#F25D94")
                .inherit(&panel)
                .padding((0, 1))
                .margin((0, 1)),
            title: Style::new()
                .bold(true)
                .foreground("#FFFDF5")
                .background("#F25D94")
                .padding((0, 1))
                .width(40)
                .align(Position::CENTER)
                .margin_bottom(1),
            gauge: Style::new()
                .border(Border::double())
                .border_foreground("#04B575")
                .margin((0, 1)),
            panel,
        }
    }
}

/// Which list item (if any) is under a click, given the list's content area
/// from the last render and its scroll offset.
fn item_at(list_area: Rect, offset: usize, len: usize, column: u16, row: u16) -> Option<usize> {
    if !list_area.contains(ratatui::layout::Position::new(column, row)) {
        return None;
    }
    let i = offset + usize::from(row - list_area.y);
    (i < len).then_some(i)
}

struct App {
    styles: Styles,
    state: ListState,
    /// Content area of the list from the last render, for mouse hit tests.
    list_area: Rect,
}

impl App {
    fn draw(&mut self, f: &mut Frame) {
        let [title, body, gauge, help] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(5),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .areas(f.area());
        // A rendered Text is itself a widget.
        f.render_widget(self.styles.title.render("ratatui-lipgloss widgets"), title);

        let [left, right] =
            Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                .areas(body);

        // StatefulWidget: wrap with Style::wrap and render with state.
        let list = List::new(ITEMS)
            .highlight_symbol("➜ ")
            .highlight_style(Style::new().foreground("#F25D94").bold(true));
        self.list_area = self.styles.focused.inner(left);
        f.render_stateful_widget(self.styles.focused.wrap(list), left, &mut self.state);

        // Widget: `.styled(&style)` comes from WidgetStyleExt.
        let text = "Lip Gloss takes an expressive, declarative approach to terminal \
                    rendering. Users familiar with CSS will feel at home.\n\n\
                    Here the borders, padding and margins around ratatui widgets \
                    come from ratatui-lipgloss styles.";
        f.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: true })
                .styled(&self.styles.panel),
            right,
        );

        let selected = self.state.selected().unwrap_or(0);
        #[allow(clippy::cast_precision_loss)]
        let ratio = (selected + 1) as f64 / ITEMS.len() as f64;
        f.render_widget(
            Gauge::default()
                .ratio(ratio)
                .gauge_style(Style::new().foreground("#04B575"))
                .styled(&self.styles.gauge),
            gauge,
        );

        let key = Style::new().foreground("#F25D94").bold(true);
        let muted = Style::new().foreground("#888888");
        f.render_widget(
            Line::from(vec![
                key.span(" q/Esc"),
                muted.span(" quit · "),
                key.span("↑/↓ j/k"),
                muted.span(" select · "),
                key.span("click"),
                muted.span(" an item to select it"),
            ]),
            help,
        );
    }
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = App {
        styles: Styles::new(),
        state: ListState::default().with_selected(Some(0)),
        list_area: Rect::default(),
    };
    loop {
        terminal.draw(|f| app.draw(f))?;
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(());
                }
                KeyCode::Down | KeyCode::Char('j') => app.state.select_next(),
                KeyCode::Up | KeyCode::Char('k') => app.state.select_previous(),
                _ => {}
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    let offset = app.state.offset();
                    if let Some(i) = item_at(app.list_area, offset, ITEMS.len(), m.column, m.row) {
                        app.state.select(Some(i));
                    }
                }
                MouseEventKind::ScrollDown => app.state.select_next(),
                MouseEventKind::ScrollUp => app.state.select_previous(),
                _ => {}
            },
            _ => {}
        }
    }
}

fn main() -> io::Result<()> {
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

    #[test]
    fn click_maps_to_item() {
        let area = Rect::new(3, 5, 10, 4);
        assert_eq!(item_at(area, 0, 6, 3, 5), Some(0));
        assert_eq!(item_at(area, 0, 6, 12, 8), Some(3));
        assert_eq!(item_at(area, 2, 6, 4, 8), Some(5));
        assert_eq!(item_at(area, 3, 6, 4, 8), None); // past the end
        assert_eq!(item_at(area, 0, 6, 2, 5), None); // left of the list
        assert_eq!(item_at(area, 0, 6, 4, 9), None); // below the list
    }

    #[test]
    fn draws_help_with_quit_key() {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 20)).unwrap();
        let mut app = App {
            styles: Styles::new(),
            state: ListState::default().with_selected(Some(0)),
            list_area: Rect::default(),
        };
        terminal.draw(|f| app.draw(f)).unwrap();
        let buf = terminal.backend().buffer();
        let last: String = (0..80).map(|x| buf[(x, 19)].symbol().to_string()).collect();
        assert!(last.starts_with(" q/Esc quit"), "{last:?}");
        // the list content area lines up with the rendered first item
        let a = app.list_area;
        let row: String = (a.x..a.right())
            .map(|x| buf[(x, a.y)].symbol().to_string())
            .collect();
        assert!(row.contains("Glossier"), "{row:?}");
    }
}
