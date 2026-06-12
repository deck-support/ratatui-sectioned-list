//! Batteries-included demo. Run with:
//!
//! ```sh
//! cargo run --example preset
//! ```
//!
//! The whole point: a working, scrollable, collapsible, focusable sectioned
//! list with *no* custom data type and *no* render closure. `BasicItem` holds
//! the text, `SectionedListWidget::basic` supplies the default styling, and
//! `SectionedListState` owns focus and input handling. Compare with
//! `ratatui_sidebar.rs`, which wires up a custom `T` and render closure by hand.

use std::io;

use ratatui::{
    crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton,
            MouseEventKind,
        },
        execute,
    },
    style::Color,
    DefaultTerminal,
};
use ratatui_sectioned_list::widget::{BasicItem, SectionedListState, SectionedListWidget};
use ratatui_sectioned_list::SectionedList;

fn build_list() -> SectionedList<BasicItem> {
    let mut list = SectionedList::new();

    // `push_header_auto` defaults the header to one row; `push_row_auto` sizes
    // each row from its text (1 + number of extra lines) — no hand-counted
    // heights. (Use `push_header` / `push_row` with an explicit height when you
    // want something else, e.g. a header with blank-row margin.)
    list.push_header_auto(BasicItem::new("local").separator("─").color(Color::Cyan));
    list.push_row_auto(
        BasicItem::new("session-a")
            .line("idle 3m")
            .line("2 windows"),
    );
    list.push_row_auto(BasicItem::new("session-b").line("running build"));

    // `push_header_margin` adds blank rows above the bar to separate sections;
    // clicks on that margin are inert — only the bar toggles the section.
    // `.button(icon)` adds a right-aligned `[icon]` button.
    list.push_header_margin(
        BasicItem::new("alice@host")
            .separator("══")
            .color(Color::Yellow)
            .button("⟳"), // refresh
        2,
    );
    list.push_row_auto(BasicItem::new("session-c"));
    list.push_row_auto(BasicItem::new("session-d").line("disconnected"));

    list.push_header_margin(
        BasicItem::new("bob@host")
            .separator("══")
            .color(Color::Green)
            .button("⟳") // refresh
            .button("⋯"), // more
        2,
    );
    list.push_row_auto(BasicItem::new("session-e"));
    list.push_row_auto(BasicItem::new("session-f").line("building"));

    list.set_collapsible(true);
    list
}

fn main() -> io::Result<()> {
    let list = build_list();
    let mut state = SectionedListState::new();
    // Mouse capture isn't part of `ratatui::run`'s defaults — enable it so
    // clicks and the wheel reach us, then turn it back off on exit.
    ratatui::run(|terminal| {
        execute!(terminal.backend_mut(), EnableMouseCapture)?;
        let result = run(terminal, &list, &mut state);
        execute!(terminal.backend_mut(), DisableMouseCapture)?;
        result
    })
}

fn run(
    terminal: &mut DefaultTerminal,
    list: &SectionedList<BasicItem>,
    state: &mut SectionedListState,
) -> io::Result<()> {
    // Click handling needs a mutable list to toggle sections, so keep a local
    // mutable copy for input while rendering borrows it immutably each frame.
    let mut list = list.clone();
    loop {
        terminal.draw(|frame| {
            let widget = SectionedListWidget::basic(&list);
            frame.render_stateful_widget(widget, frame.area(), state);
        })?;
        match event::read()? {
            Event::Key(k) => match k.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('j') | KeyCode::Down => state.move_focus(&list, 1),
                KeyCode::Char('k') | KeyCode::Up => state.move_focus(&list, -1),
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.toggle_focused_section(&mut list);
                }
                _ => {}
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    // `handle_click_basic` toggles/focuses, and returns the
                    // clicked header button (if any) without toggling. This demo
                    // doesn't act on buttons, but routing through it keeps a
                    // button click from collapsing the section.
                    let _ = state.handle_click_basic(&mut list, m.column, m.row);
                }
                MouseEventKind::ScrollDown => state.move_focus(&list, 1),
                MouseEventKind::ScrollUp => state.move_focus(&list, -1),
                _ => {}
            },
            _ => {}
        }
    }
}
