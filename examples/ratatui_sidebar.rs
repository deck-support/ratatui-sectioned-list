//! Interactive demo built on the **`BasicItem` preset** — the same default
//! styling and focus highlight as `preset`, plus two things `preset` doesn't
//! show: `+`/`-` to resize the focused row, and a right pane that reads
//! `scroll_offset`, `row_y`, and `locate_row` live.
//!
//! ```sh
//! cargo run --example ratatui_sidebar
//! ```

use std::io;

use ratatui::{
    crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton,
            MouseEventKind,
        },
        execute,
    },
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    DefaultTerminal, Frame,
};
use ratatui_sectioned_list::widget::{BasicItem, SectionedListState, SectionedListWidget};
use ratatui_sectioned_list::{RowLocation, SectionedList};

fn build_list() -> SectionedList<BasicItem> {
    let mut list = SectionedList::new();

    // Left-aligned label, full-width `─` divider, two right-side buttons.
    list.push_header_auto(
        BasicItem::new("local")
            .separator("─")
            .color(Color::Cyan)
            .button("⟳") // refresh → [⟳]
            .button("⋯"), // more    → [⋯]
    );
    list.push_row_auto(
        BasicItem::new("session-a")
            .line("idle 3m")
            .line("2 windows"),
    );
    list.push_row_auto(BasicItem::new("session-b").line("running build"));
    list.push_row_auto(BasicItem::new("press + / - to resize me"));

    // `push_header_margin` adds blank rows above the bar to separate sections —
    // here 2 rows of top margin. Centered label, plus the same two buttons.
    list.push_header_margin(
        BasicItem::new("remote: alice@host")
            .separator("══")
            .color(Color::Yellow)
            .align(Alignment::Center)
            .button("⟳")
            .button("⋯"),
        2,
    );
    list.push_row_auto(BasicItem::new("session-c").line("connected"));
    list.push_row_auto(BasicItem::new("session-d"));

    list.set_collapsible(true);
    list
}

struct App {
    list: SectionedList<BasicItem>,
    state: SectionedListState,
    last_action: String,
}

impl App {
    fn new() -> Self {
        Self {
            list: build_list(),
            state: SectionedListState::new(),
            last_action: "—".to_string(),
        }
    }

    /// Grow/shrink the focused row, clamped to 1..=10. Row-height editing is app
    /// data, not list geometry, so it lives here rather than in the widget.
    fn resize_focused(&mut self, delta: i32) {
        let focused = self.state.focused();
        let Some((top, bottom)) = self.list.row_y(focused) else {
            return;
        };
        let next = ((bottom - top) as i32 + delta).clamp(1, 10) as u16;
        self.list.set_row_height(focused, next);
    }
}

fn main() -> io::Result<()> {
    let mut app = App::new();
    // `ratatui::run` handles raw mode, the alternate screen, and a panic hook.
    // Mouse capture isn't in those defaults, so we toggle it around the loop.
    ratatui::run(|terminal| {
        execute!(terminal.backend_mut(), EnableMouseCapture)?;
        let result = run(terminal, &mut app);
        execute!(terminal.backend_mut(), DisableMouseCapture)?;
        result
    })
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, app))?;
        match event::read()? {
            Event::Key(k) => match k.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('j') | KeyCode::Down => app.state.move_focus(&app.list, 1),
                KeyCode::Char('k') | KeyCode::Up => app.state.move_focus(&app.list, -1),
                KeyCode::Char('+') | KeyCode::Char('=') => app.resize_focused(1),
                KeyCode::Char('-') | KeyCode::Char('_') => app.resize_focused(-1),
                KeyCode::Enter | KeyCode::Char(' ') => {
                    app.state.toggle_focused_section(&mut app.list);
                }
                _ => {}
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    // `handle_click_basic` toggles/focuses as usual, but returns
                    // the button when a header button is clicked (no toggle then).
                    if let Some((section, button)) =
                        app.state.handle_click_basic(&mut app.list, m.column, m.row)
                    {
                        let name = if button == 0 { "refresh" } else { "more" };
                        app.last_action = format!("{name} on section {section}");
                    }
                }
                MouseEventKind::ScrollDown => app.state.move_focus(&app.list, 1),
                MouseEventKind::ScrollUp => app.state.move_focus(&app.list, -1),
                _ => {}
            },
            _ => {}
        }
    }
}

fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(60), Constraint::Min(30)])
        .split(frame.area());

    let sidebar = Block::default()
        .borders(Borders::ALL)
        .title(" sectioned list ");
    let inner = sidebar.inner(chunks[0]);
    frame.render_widget(sidebar, chunks[0]);

    // The preset: built-in `BasicItem` styling plus the default focus highlight.
    frame.render_stateful_widget(SectionedListWidget::basic(&app.list), inner, &mut app.state);

    let right = Block::default().borders(Borders::ALL).title(" details ");
    let right_inner = right.inner(chunks[1]);
    frame.render_widget(right, chunks[1]);
    frame.render_widget(
        Paragraph::new(details(app, app.state.scroll_offset(), inner.height)),
        right_inner,
    );
}

fn details(app: &App, scroll: u16, viewport_h: u16) -> Text<'static> {
    let focused = app.state.focused();
    let section = match app.list.locate_row(focused) {
        Some(RowLocation {
            section: Some(s),
            row_in_section,
        }) => format!("{s} (row {row_in_section})"),
        Some(RowLocation {
            section: None,
            row_in_section,
        }) => format!("(none) (row {row_in_section})"),
        None => "-".to_string(),
    };
    let row_y = match app.list.row_y(focused) {
        Some((t, b)) => format!("({t}, {b})"),
        None => "None".to_string(),
    };
    let dim = Style::default().fg(Color::DarkGray);
    Text::from(vec![
        Line::from(format!("focused row  : {focused}")),
        Line::from(format!("section      : {section}")),
        Line::from(format!("row_y        : {row_y}")),
        Line::from(format!("scroll_offset: {scroll}")),
        Line::from(format!("viewport h   : {viewport_h}")),
        Line::from(format!(
            "rows / height: {} / {}",
            app.list.row_count(),
            app.list.total_height()
        )),
        Line::from(format!("last button  : {}", app.last_action)),
        Line::from(""),
        Line::from(Span::styled("j/k move · enter collapse · +/- resize", dim)),
        Line::from(Span::styled(
            "click row/divider/[⟳][⋯] · wheel · q quit",
            dim,
        )),
    ])
}
