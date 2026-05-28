//! Interactive ratatui demo. Run with:
//!
//! ```sh
//! cargo run --example ratatui_sidebar
//! ```
//!
//! Sidebar of sessions grouped by host. Variable row heights. Keyboard
//! navigation (j/k or arrow keys) and mouse click to focus a row. The
//! right pane displays the current focus + scroll state so you can see
//! `scroll_offset` and `row_at_y` in action.

use std::io::{self, Stdout};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use ratatui_sectioned_list::{ItemKind, RowLocation, SectionedList};

struct Row {
    label: &'static str,
    detail: &'static str,
    status: &'static str,
    tags: &'static str,
    // Header-only styling. Rows ignore these fields.
    header_color: Color,
    header_sep: &'static str,
}

const fn header(label: &'static str, color: Color, sep: &'static str) -> Row {
    Row {
        label,
        detail: "",
        status: "",
        tags: "",
        header_color: color,
        header_sep: sep,
    }
}

const fn row(
    label: &'static str,
    detail: &'static str,
    status: &'static str,
    tags: &'static str,
) -> Row {
    Row {
        label,
        detail,
        status,
        tags,
        header_color: Color::Reset,
        header_sep: "",
    }
}

fn build_list() -> SectionedList<Row> {
    let mut list = SectionedList::new();

    // Section 1: cyan single-line separator, header h=1 (no margin).
    list.push_header(header("section 1: cyan divider, h=1", Color::Cyan, "─"), 1);
    list.push_row(
        row(
            "row h=4 — 4 content lines",
            "line 2: detail field",
            "line 3: status field",
            "line 4: tags field",
        ),
        4,
    );
    list.push_row(
        row("row h=2 — 2 content lines", "line 2: detail only", "", ""),
        2,
    );
    list.push_row(
        row(
            "row h=3 — 3 content lines",
            "line 2: detail",
            "line 3: status",
            "",
        ),
        3,
    );

    // Section 2: yellow double-line separator, header h=3 → blank above + below.
    list.push_header(
        header(
            "section 2: yellow ══, h=3 → margin top + bottom",
            Color::Yellow,
            "══",
        ),
        3,
    );
    list.push_row(
        row(
            "the blank rows above/below the header",
            "come from header height = 3",
            "renderer puts (h-1)/2 blanks above the bar",
            "",
        ),
        4,
    );
    list.push_row(
        row(
            "press + / - to resize this row",
            "wired via set_row_height(global_idx, h)",
            "",
            "",
        ),
        3,
    );
    list.push_row(row("press j / k to move focus", "", "", ""), 2);

    // Section 3: green dotted separator, header h=1.
    list.push_header(
        header("section 3: green · · · divider, h=1", Color::Green, "· · ·"),
        1,
    );
    list.push_row(
        row(
            "click any row to focus it",
            "row_at_y(viewport_y, scroll)",
            "returns global row index",
            "",
        ),
        4,
    );
    list.push_row(
        row(
            "scroll follows focus automatically",
            "scroll_offset(focused, viewport_h)",
            "anchors focused row's bottom to viewport bottom",
            "",
        ),
        4,
    );
    list.push_row(row("press q or Esc to quit", "", "", ""), 2);

    list
}

struct App {
    list: SectionedList<Row>,
    focused: usize,
    sidebar_inner: Rect,
}

impl App {
    fn new() -> Self {
        Self {
            list: build_list(),
            focused: 0,
            sidebar_inner: Rect::default(),
        }
    }

    fn move_focus(&mut self, delta: i32) {
        let n = self.list.row_count() as i32;
        if n == 0 {
            return;
        }
        let new = (self.focused as i32 + delta).rem_euclid(n);
        self.focused = new as usize;
    }

    fn resize_focused(&mut self, delta: i32) {
        let Some((_, bottom)) = self.list.row_y(self.focused) else {
            return;
        };
        let (top, _) = self.list.row_y(self.focused).unwrap();
        let current = bottom - top;
        let next = (current as i32 + delta).clamp(1, 10) as u16;
        self.list.set_row_height(self.focused, next);
    }

    fn handle_click(&mut self, col: u16, row: u16) {
        let r = self.sidebar_inner;
        if col < r.x || col >= r.x + r.width {
            return;
        }
        if row < r.y || row >= r.y + r.height {
            return;
        }
        let viewport_y = row - r.y;
        let scroll = self.list.scroll_offset(Some(self.focused), r.height);
        if let Some(idx) = self.list.row_at_y(viewport_y, scroll) {
            self.focused = idx;
        }
    }
}

fn main() -> io::Result<()> {
    let mut terminal = setup()?;
    let mut app = App::new();
    let result = run(&mut terminal, &mut app);
    teardown()?;
    result
}

fn setup() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn teardown() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        terminal
            .draw(|frame| draw(frame, app))
            .map_err(io::Error::other)?;
        match event::read()? {
            Event::Key(k) => match k.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('j') | KeyCode::Down => app.move_focus(1),
                KeyCode::Char('k') | KeyCode::Up => app.move_focus(-1),
                KeyCode::Char('+') | KeyCode::Char('=') => app.resize_focused(1),
                KeyCode::Char('-') | KeyCode::Char('_') => app.resize_focused(-1),
                _ => {}
            },
            Event::Mouse(m) if matches!(m.kind, MouseEventKind::Down(MouseButton::Left)) => {
                app.handle_click(m.column, m.row);
            }
            _ => {}
        }
    }
}

fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(60), Constraint::Min(30)])
        .split(frame.area());

    let sidebar_block = Block::default()
        .borders(Borders::ALL)
        .title(" sectioned list ");
    let inner = sidebar_block.inner(chunks[0]);
    app.sidebar_inner = inner;
    frame.render_widget(sidebar_block, chunks[0]);

    let scroll = app.list.scroll_offset(Some(app.focused), inner.height);
    draw_sidebar(frame, app, inner, scroll);

    let details = build_details(app, scroll, inner.height);
    let right = Block::default().borders(Borders::ALL).title(" details ");
    let right_inner = right.inner(chunks[1]);
    frame.render_widget(right, chunks[1]);
    frame.render_widget(Paragraph::new(details), right_inner);
}

fn draw_sidebar(frame: &mut Frame, app: &App, inner: Rect, scroll: u16) {
    for v in app.list.visible_items(scroll, inner.height) {
        let cell = Rect {
            x: inner.x,
            y: inner.y + v.viewport_y,
            width: inner.width,
            height: v.visible_height,
        };
        let text = match v.item.kind {
            ItemKind::Header => header_lines(&v.item.data, v.item.height),
            ItemKind::Row => row_lines(&v.item.data, v.row_idx == Some(app.focused)),
        };
        frame.render_widget(Paragraph::new(text), cell);
    }
}

fn header_lines(h: &Row, height: u16) -> Vec<Line<'static>> {
    let bar = Line::from(Span::styled(
        format!("{0} {1} {0}", h.header_sep, h.label),
        Style::default()
            .fg(h.header_color)
            .add_modifier(Modifier::BOLD),
    ));
    if height <= 1 {
        return vec![bar];
    }
    // Center the bar vertically; blank cells above + below act as margin.
    let top = (height - 1) / 2;
    let mut lines: Vec<Line<'static>> = Vec::with_capacity(height as usize);
    for _ in 0..top {
        lines.push(Line::from(""));
    }
    lines.push(bar);
    while (lines.len() as u16) < height {
        lines.push(Line::from(""));
    }
    lines
}

fn row_lines(row: &Row, focused: bool) -> Vec<Line<'static>> {
    let title_style = if focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let bullet = if focused { "▌ " } else { "  " };
    let dim = Style::default().fg(Color::DarkGray);
    let mut lines = vec![Line::from(vec![
        Span::styled(bullet.to_string(), title_style),
        Span::styled(row.label.to_string(), title_style),
    ])];
    if !row.detail.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(row.detail.to_string(), dim),
        ]));
    }
    if !row.status.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(row.status.to_string(), dim),
        ]));
    }
    if !row.tags.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(row.tags.to_string(), dim),
        ]));
    }
    lines
}

fn build_details(app: &App, scroll: u16, viewport_h: u16) -> Vec<Line<'static>> {
    let focused_y = app.list.row_y(app.focused);
    let location = app.list.locate_row(app.focused);
    let dim = Style::default().fg(Color::DarkGray);

    let (section_label, row_in_section_label) = match location {
        Some(RowLocation {
            section,
            row_in_section,
        }) => {
            let section = section
                .map(|s| s.to_string())
                .unwrap_or_else(|| "(none)".to_string());
            (section, row_in_section.to_string())
        }
        None => ("-".to_string(), "-".to_string()),
    };

    vec![
        Line::from(format!("global index       : {}", app.focused)),
        Line::from(format!("section index      : {section_label}")),
        Line::from(format!("row-in-section idx : {row_in_section_label}")),
        Line::from(""),
        Line::from(format!(
            "row_y(global)      : {}",
            match focused_y {
                Some((t, b)) => format!("({t}, {b})"),
                None => "None".to_string(),
            }
        )),
        Line::from(format!("scroll_offset      : {scroll}")),
        Line::from(format!("viewport height    : {viewport_h}")),
        Line::from(format!("row_count          : {}", app.list.row_count())),
        Line::from(format!("total_height       : {}", app.list.total_height())),
        Line::from(""),
        Line::from(Span::styled("keys", dim)),
        Line::from(Span::styled("  j / ↓     move focus down", dim)),
        Line::from(Span::styled("  k / ↑     move focus up", dim)),
        Line::from(Span::styled(
            "  + / -     grow / shrink focused row (1..10)",
            dim,
        )),
        Line::from(Span::styled("  click     focus row under cursor", dim)),
        Line::from(Span::styled("  q / Esc   quit", dim)),
    ]
}
