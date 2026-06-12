//! Optional ratatui [`StatefulWidget`] wrapper around [`SectionedList`].
//!
//! Available only with the `ratatui` feature. The core crate stays
//! framework-agnostic; this module is the convenience layer for ratatui apps
//! that don't want to hand-write the `visible_items` → `Rect` render loop.
//!
//! You still own *how a row looks*: pass a closure that turns an [`Item`] into
//! ratatui [`Text`]. The widget owns the geometry (scroll, clipping, hit-test)
//! and [`SectionedListState`] owns focus.
//!
//! ```no_run
//! use ratatui::{text::Text, Frame};
//! use ratatui_sectioned_list::{ItemKind, SectionedList};
//! use ratatui_sectioned_list::widget::{SectionedListWidget, SectionedListState};
//!
//! fn draw(frame: &mut Frame, list: &SectionedList<&str>, state: &mut SectionedListState) {
//!     let widget = SectionedListWidget::new(list, |item, ctx| {
//!         let prefix = if ctx.focused { "> " } else { "  " };
//!         Text::raw(format!("{prefix}{}", item.data))
//!     });
//!     frame.render_stateful_widget(widget, frame.area(), state);
//! }
//! ```

use std::ops::Range;

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, StatefulWidget, Widget};

use crate::{Item, ItemKind, RowHeight, SectionedList};

/// Per-item context handed to the render closure.
///
/// Everything here is derived from the list and the current focus, so the
/// closure can style headers, the focused row, and collapse indicators without
/// reaching back into the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemContext {
    /// `Some(row_idx)` for focusable rows (global index); `None` for headers.
    pub row_idx: Option<usize>,
    /// Whether this row is the focused one. Always `false` for headers.
    pub focused: bool,
    /// Whether the list has collapsing enabled — draw a ▾/▸ indicator on
    /// headers only when this is `true`.
    pub collapsible: bool,
    /// The item's stored collapsed flag (meaningful for headers).
    pub collapsed: bool,
    /// The cell width this item renders into — useful for full-width dividers
    /// and right-aligned content.
    pub width: u16,
}

/// Focus + viewport state for [`SectionedListWidget`].
///
/// Stored by the app and passed to `render_stateful_widget`. The widget writes
/// the last rendered area and scroll offset back into it on every render, so
/// the click/scroll helpers can map terminal coordinates without the app
/// threading geometry around.
#[derive(Debug, Clone, Default)]
pub struct SectionedListState {
    focused: usize,
    last_area: Rect,
    last_scroll: u16,
}

impl SectionedListState {
    pub fn new() -> Self {
        Self::default()
    }

    /// The global index of the focused row.
    pub fn focused(&self) -> usize {
        self.focused
    }

    /// Set the focused row directly (e.g. to restore a selection). Not range
    /// checked — an out-of-range value simply renders nothing as focused until
    /// the next [`move_focus`](Self::move_focus).
    pub fn set_focused(&mut self, row_idx: usize) {
        self.focused = row_idx;
    }

    /// The scroll offset computed at the last render. `0` before the first
    /// render.
    pub fn scroll_offset(&self) -> u16 {
        self.last_scroll
    }

    /// Step focus by `delta` rows (negative = up), wrapping around the ends and
    /// skipping rows hidden inside a collapsed section. No-op when every row is
    /// hidden or the list has no rows.
    pub fn move_focus<T>(&mut self, list: &SectionedList<T>, delta: i32) {
        let n = list.row_count() as i32;
        if n == 0 {
            return;
        }
        let mut candidate = self.focused as i32;
        for _ in 0..n {
            candidate = (candidate + delta).rem_euclid(n);
            if !list.is_row_hidden(candidate as usize) {
                self.focused = candidate as usize;
                return;
            }
        }
    }

    /// If the focused row is currently hidden by a collapse, move focus to the
    /// nearest following visible row. Call this after toggling a section.
    pub fn ensure_focus_visible<T>(&mut self, list: &SectionedList<T>) {
        if list.is_row_hidden(self.focused) {
            self.move_focus(list, 1);
        }
    }

    /// Toggle the section that contains the focused row. Returns `true` if a
    /// section was toggled.
    ///
    /// Focus is left where it is: collapsing the focused row's section hides
    /// that row but keeps it selected, so re-expanding restores the selection
    /// in place rather than jumping to a neighbor. Call
    /// [`ensure_focus_visible`](Self::ensure_focus_visible) yourself if you
    /// prefer focus to follow the visible rows.
    pub fn toggle_focused_section<T>(&mut self, list: &mut SectionedList<T>) -> bool {
        let Some(crate::RowLocation {
            section: Some(s), ..
        }) = list.locate_row(self.focused)
        else {
            return false;
        };
        list.toggle_section(s)
    }

    /// Handle a left-click at absolute terminal coordinates `(col, row)`.
    ///
    /// Uses the area and scroll offset recorded at the last render. A click on
    /// a divider toggles its section (when collapsing is enabled); a click on a
    /// row focuses it. Coordinates outside the last rendered area are ignored.
    /// Returns `true` if the click hit a row or header.
    pub fn handle_click<T>(&mut self, list: &mut SectionedList<T>, col: u16, row: u16) -> bool {
        let area = self.last_area;
        if col < area.x
            || col >= area.x.saturating_add(area.width)
            || row < area.y
            || row >= area.y.saturating_add(area.height)
        {
            return false;
        }
        let viewport_y = row - area.y;
        if list.is_collapsible() {
            if let Some(section) = list.header_at_y(viewport_y, self.last_scroll) {
                // Toggle only; focus stays put even if its row is now hidden, so
                // collapsing never silently reselects a different row.
                list.toggle_section(section);
                return true;
            }
        }
        if let Some(idx) = list.row_at_y(viewport_y, self.last_scroll) {
            self.focused = idx;
            return true;
        }
        false
    }
}

/// A ratatui [`StatefulWidget`] that renders a [`SectionedList`] into a `Rect`.
///
/// Construct with [`new`](Self::new), supplying a closure that maps each
/// [`Item`] (plus its [`ItemContext`]) to ratatui [`Text`]. The widget folds in
/// scroll, viewport clipping, and per-item placement; the closure owns
/// appearance.
pub struct SectionedListWidget<'a, T, F> {
    list: &'a SectionedList<T>,
    render_item: F,
    highlight_style: Style,
}

impl<'a, T, F> SectionedListWidget<'a, T, F>
where
    F: Fn(&Item<T>, ItemContext) -> Text<'static>,
{
    /// Wrap `list` with a per-item render closure. No focus highlight by
    /// default — add one with [`highlight_style`](Self::highlight_style).
    pub fn new(list: &'a SectionedList<T>, render_item: F) -> Self {
        Self {
            list,
            render_item,
            highlight_style: Style::default(),
        }
    }

    /// Style painted across the focused row's full cell, like ratatui's
    /// `List::highlight_style`. Use it for a background bar behind the focused
    /// row; the closure's own per-span foreground styling renders on top.
    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }
}

impl<T, F> StatefulWidget for SectionedListWidget<'_, T, F>
where
    F: Fn(&Item<T>, ItemContext) -> Text<'static>,
{
    type State = SectionedListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let scroll = self.list.scroll_offset(Some(state.focused), area.height);
        state.last_area = area;
        state.last_scroll = scroll;

        for v in self.list.visible_items(scroll, area.height) {
            let cell = Rect {
                x: area.x,
                y: area.y + v.viewport_y,
                width: area.width,
                height: v.visible_height,
            };
            let ctx = ItemContext {
                row_idx: v.row_idx,
                focused: v.row_idx == Some(state.focused),
                collapsible: self.list.is_collapsible(),
                collapsed: v.item.collapsed,
                width: cell.width,
            };
            // Paint the highlight across the whole focused cell first; the text
            // spans render on top and keep their own foreground.
            if ctx.focused {
                buf.set_style(cell, self.highlight_style);
            }
            let text = (self.render_item)(v.item, ctx);
            Paragraph::new(text).render(cell, buf);
        }
    }
}

/// A batteries-included payload for apps that just want styled text rows and
/// don't need a custom `T` or render closure. Pair it with [`basic_style`] (or
/// the [`SectionedListWidget::basic`] shortcut).
///
/// One type serves both headers and rows — whether an instance is a header or a
/// row is decided by [`SectionedList::push_header`] / [`push_row`], not by a
/// field here. Build with the chained setters:
///
/// ```
/// # use ratatui_sectioned_list::widget::BasicItem;
/// # use ratatui::{layout::Alignment, style::Color};
/// let header = BasicItem::new("local")
///     .separator("─")
///     .color(Color::Cyan)
///     .align(Alignment::Center)
///     .button("⟳")  // refresh, rendered as [⟳]
///     .button("⋯"); // more,    rendered as [⋯]
/// let row = BasicItem::new("session-a").line("idle 3m").line("2 windows");
/// ```
///
/// [`push_row`]: SectionedList::push_row
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicItem {
    /// Header label or row title — the first rendered line.
    pub title: String,
    /// Extra dim lines under the title (rows only). Each takes one terminal
    /// row, so keep the pushed `height` in sync (1 + `lines.len()`).
    pub lines: Vec<String>,
    /// Divider fill repeated to span the header's full width on either side of
    /// the label (e.g. `"─"`, `"══"`). Empty fills with spaces. Ignored for rows.
    pub separator: String,
    /// Accent color: the header bar's foreground, and the focus marker on rows.
    pub color: Color,
    /// Horizontal alignment of the header label within the divider. Ignored for
    /// rows (always left-aligned).
    pub align: Alignment,
    /// Right-aligned header buttons, each rendered as `[icon]` in the order
    /// added. Ignored for rows. Hit-test clicks with
    /// [`SectionedListState::handle_click_basic`].
    pub buttons: Vec<String>,
}

impl BasicItem {
    /// A new item with the given title, no extra lines, no separator, left
    /// alignment, no buttons, and the terminal's default color.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            lines: Vec::new(),
            separator: String::new(),
            color: Color::Reset,
            align: Alignment::Left,
            buttons: Vec::new(),
        }
    }

    /// Append a secondary line (rows only). Chainable.
    pub fn line(mut self, text: impl Into<String>) -> Self {
        self.lines.push(text.into());
        self
    }

    /// Set the header divider fill. Chainable.
    pub fn separator(mut self, sep: impl Into<String>) -> Self {
        self.separator = sep.into();
        self
    }

    /// Set the accent color. Chainable.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the header label alignment (left/center/right). Chainable.
    pub fn align(mut self, align: Alignment) -> Self {
        self.align = align;
        self
    }

    /// Append a right-aligned header button, rendered as `[icon]`. Chainable;
    /// buttons appear left-to-right in the order added. The icon can be any
    /// string (a glyph, a short label).
    pub fn button(mut self, icon: impl Into<String>) -> Self {
        self.buttons.push(format!("[{}]", icon.into()));
        self
    }
}

impl RowHeight for BasicItem {
    /// The title line plus one row per extra line. Pair with
    /// [`SectionedList::push_row_auto`] to size rows from their text.
    fn row_height(&self) -> u16 {
        1 + self.lines.len() as u16
    }
}

/// The default render preset for [`BasicItem`]: bold colored header bars with a
/// ▾/▸ collapse indicator (when collapsing is enabled), and rows whose title is
/// highlighted when focused with dim secondary lines beneath.
///
/// Pass it to [`SectionedListWidget::new`], or use [`SectionedListWidget::basic`]
/// which wires it for you.
pub fn basic_style(item: &Item<BasicItem>, ctx: ItemContext) -> Text<'static> {
    let data = &item.data;
    match item.kind {
        ItemKind::Header => {
            let bar = header_bar_line(data, ctx);
            // `item.lead` blank rows above the bar (the hit-test excludes them),
            // then the bar, then any remaining height as trailing blanks.
            let height = item.height.max(1) as usize;
            let above = (item.lead as usize).min(height - 1);
            let mut lines = vec![Line::from(""); above];
            lines.push(bar);
            lines.resize(height, Line::from(""));
            Text::from(lines)
        }
        ItemKind::Row => {
            // Focused rows are drawn on the widget's highlight background, so
            // the secondary lines use a lighter gray to stay readable there;
            // unfocused ones dim to dark gray.
            let (title_style, line_color) = if ctx.focused {
                (
                    Style::default().fg(data.color).add_modifier(Modifier::BOLD),
                    Color::Gray,
                )
            } else {
                (Style::default(), Color::DarkGray)
            };
            let marker = if ctx.focused { "▌ " } else { "  " };
            let mut lines = vec![Line::from(vec![
                Span::styled(marker.to_string(), title_style),
                Span::styled(data.title.clone(), title_style),
            ])];
            for extra in &data.lines {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(extra.clone(), Style::default().fg(line_color)),
                ]));
            }
            Text::from(lines)
        }
    }
}

/// Display width of `s` in terminal cells.
fn display_width(s: &str) -> u16 {
    Span::raw(s).width() as u16
}

/// Truncate `s` to at most `width` display cells.
fn truncate_to_width(s: &str, width: u16) -> String {
    let mut out = String::new();
    let mut w = 0u16;
    for ch in s.chars() {
        let cw = display_width(&ch.to_string());
        if w + cw > width {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out
}

/// Repeat `sep` (or spaces, if empty) to exactly `width` display cells.
fn fill_str(sep: &str, width: u16) -> String {
    if width == 0 {
        return String::new();
    }
    if sep.is_empty() {
        return " ".repeat(width as usize);
    }
    let unit = display_width(sep).max(1);
    let reps = (width / unit + 1) as usize;
    truncate_to_width(&sep.repeat(reps), width)
}

/// Right-aligned `[icon]` button cell ranges within a header `width` cells wide,
/// in button order. A 1-cell gap separates the buttons (and precedes the first).
/// Returns empty if the buttons don't fit. Shared by render and hit-testing so
/// the two always agree.
fn header_button_ranges(width: u16, buttons: &[String]) -> Vec<Range<u16>> {
    if buttons.is_empty() {
        return Vec::new();
    }
    let widths: Vec<u16> = buttons.iter().map(|b| display_width(b)).collect();
    let total: u16 = widths.iter().sum::<u16>() + (buttons.len() as u16 - 1);
    if total > width {
        return Vec::new();
    }
    let mut x = width - total;
    let mut out = Vec::with_capacity(buttons.len());
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            x += 1;
        }
        let range = x..x + w;
        x = range.end;
        out.push(range);
    }
    out
}

/// Build the single full-width header bar line: chevron, the label aligned and
/// padded with the separator fill to span the width, then right-aligned buttons.
fn header_bar_line(data: &BasicItem, ctx: ItemContext) -> Line<'static> {
    let accent = Style::default().fg(data.color);
    let bold = accent.add_modifier(Modifier::BOLD);
    let width = ctx.width;

    let chevron = match (ctx.collapsible, ctx.collapsed) {
        (false, _) => "",
        (true, true) => "▸ ",
        (true, false) => "▾ ",
    };
    let chevron_w = display_width(chevron);

    let buttons = header_button_ranges(width, &data.buttons);
    // Button block width + a 1-cell gap separating it from the fill.
    let (buttons_w, gap_before) = match buttons.first() {
        Some(r) => (width - r.start, 1),
        None => (0, 0),
    };

    let mid = width
        .saturating_sub(chevron_w)
        .saturating_sub(buttons_w)
        .saturating_sub(gap_before);

    let label = truncate_to_width(&format!(" {} ", data.title), mid);
    let label_w = display_width(&label);
    let fill_total = mid - label_w;
    let (lpad, rpad) = match data.align {
        Alignment::Left => (0, fill_total),
        Alignment::Right => (fill_total, 0),
        Alignment::Center => (fill_total / 2, fill_total - fill_total / 2),
    };

    let mut spans: Vec<Span<'static>> = Vec::new();
    if !chevron.is_empty() {
        spans.push(Span::styled(chevron.to_string(), bold));
    }
    if lpad > 0 {
        spans.push(Span::styled(fill_str(&data.separator, lpad), accent));
    }
    if !label.is_empty() {
        spans.push(Span::styled(label, bold));
    }
    if rpad > 0 {
        spans.push(Span::styled(fill_str(&data.separator, rpad), accent));
    }
    if !data.buttons.is_empty() && !buttons.is_empty() {
        spans.push(Span::raw(" ".repeat(gap_before as usize)));
        for (i, text) in data.buttons.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(text.clone(), accent));
        }
    }
    Line::from(spans)
}

impl SectionedListState {
    /// Click handling for [`BasicItem`] lists, on top of [`handle_click`]: it
    /// also recognizes header buttons. Focuses a clicked row, toggles a section
    /// when its divider bar is clicked, and — when a header button is clicked —
    /// returns `Some((section, button))` *without* toggling, so the caller runs
    /// that button's action. Returns `None` for any other click, including the
    /// inert header margin.
    ///
    /// [`handle_click`]: Self::handle_click
    pub fn handle_click_basic(
        &mut self,
        list: &mut SectionedList<BasicItem>,
        col: u16,
        row: u16,
    ) -> Option<(usize, usize)> {
        let area = self.last_area;
        if col < area.x
            || col >= area.x.saturating_add(area.width)
            || row < area.y
            || row >= area.y.saturating_add(area.height)
        {
            return None;
        }
        let viewport_y = row - area.y;

        if let Some(section) = list.header_at_y(viewport_y, self.last_scroll) {
            let col_in = col - area.x;
            let button = list
                .items()
                .iter()
                .filter(|it| it.kind == ItemKind::Header)
                .nth(section)
                .and_then(|it| {
                    header_button_ranges(area.width, &it.data.buttons)
                        .into_iter()
                        .position(|r| col_in >= r.start && col_in < r.end)
                });
            if let Some(b) = button {
                return Some((section, b));
            }
            if list.is_collapsible() {
                list.toggle_section(section);
            }
            return None;
        }

        if let Some(idx) = list.row_at_y(viewport_y, self.last_scroll) {
            self.focused = idx;
        }
        None
    }
}

/// Convenience render fn pointer type for the [`BasicItem`] preset.
pub type BasicStyleFn = fn(&Item<BasicItem>, ItemContext) -> Text<'static>;

impl<'a> SectionedListWidget<'a, BasicItem, BasicStyleFn> {
    /// Wrap a [`BasicItem`] list with the [`basic_style`] preset — the
    /// zero-config path. Equivalent to `SectionedListWidget::new(list, basic_style)`
    /// plus a default dark-gray focus highlight; override it with
    /// [`highlight_style`](Self::highlight_style).
    pub fn basic(list: &'a SectionedList<BasicItem>) -> Self {
        Self::new(list, basic_style).highlight_style(Style::default().bg(Color::DarkGray))
    }
}

#[cfg(test)]
mod tests;
