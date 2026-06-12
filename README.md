# ratatui-sectioned-list

[![crates.io](https://img.shields.io/crates/v/ratatui-sectioned-list.svg)](https://crates.io/crates/ratatui-sectioned-list)
[![downloads](https://img.shields.io/crates/d/ratatui-sectioned-list.svg)](https://crates.io/crates/ratatui-sectioned-list)
[![docs.rs](https://img.shields.io/docsrs/ratatui-sectioned-list)](https://docs.rs/ratatui-sectioned-list)
[![CI](https://github.com/deck-support/ratatui-sectioned-list/actions/workflows/ci.yml/badge.svg)](https://github.com/deck-support/ratatui-sectioned-list/actions/workflows/ci.yml)
[![license](https://img.shields.io/crates/l/ratatui-sectioned-list.svg)](https://github.com/deck-support/ratatui-sectioned-list/blob/main/LICENSE)

A **sectioned list** for [ratatui](https://github.com/ratatui/ratatui): non-focusable section headers interleaved with focusable, variable-height rows — with scrolling, collapsible sections, and mouse hit-testing handled for you.

Ships a ready-to-use [`StatefulWidget`] plus a `BasicItem` preset, so a working list is a few lines. Underneath sits a UI-framework-agnostic geometry core (plain `u16` rows, zero dependencies) you can drop down to — or use on its own with `default-features = false`.

![interactive ratatui demo](https://raw.githubusercontent.com/deck-support/ratatui-sectioned-list/main/SCR-20260529-bvkh.png)

## When you need this

Your list has:

- **Section headers** — visible labels that aren't focusable.
- **Variable-height rows** — some rows are 1 line, some are 3.
- **Mouse hit-testing** — clicks map back to a row index (or a header, for click-to-collapse).
- **Collapsible sections** — fold a section away while keeping stable row indices.
- **Styled headers** — full-width dividers, label alignment, top margin, and clickable right-side buttons.

[`tui-widget-list`](https://crates.io/crates/tui-widget-list) is the closest existing crate, but it doesn't model the header/row distinction or expose viewport-relative hit-testing. This crate fills that gap.

## Quick start

Out of the box (the `ratatui` feature is on by default), use the `BasicItem` payload and the `basic` preset — no custom type, no render closure:

```rust
use ratatui::style::Color;
use ratatui_sectioned_list::SectionedList;
use ratatui_sectioned_list::widget::{BasicItem, SectionedListState, SectionedListWidget};

let mut list = SectionedList::new();
// Auto-height: headers default to 1 row, rows size themselves from their text.
list.push_header_auto(BasicItem::new("local").separator("─").color(Color::Cyan));
list.push_row_auto(BasicItem::new("session-a").line("idle 3m").line("2 windows")); // 3 rows
list.push_row_auto(BasicItem::new("session-b").line("running build"));             // 2 rows
list.set_collapsible(true);

let mut state = SectionedListState::new();

// In your draw closure — scroll, clipping, and styling are handled:
frame.render_stateful_widget(SectionedListWidget::basic(&list), area, &mut state);

// In your event handler:
state.move_focus(&list, 1);              // j / k / wheel — skips collapsed rows
state.toggle_focused_section(&mut list); // enter / space
state.handle_click(&mut list, col, row); // click a row to focus, a divider to collapse
```

`SectionedListState` owns focus and records the area/scroll from the last render, so the navigation and click helpers need no extra geometry from you.

## Header styling

`BasicItem` headers support a full-width divider fill, label alignment, top margin, and right-aligned `[icon]` buttons:

```rust
use ratatui::layout::Alignment;

list.push_header_margin(                 // 2 blank rows above (inert — clicks there do nothing)
    BasicItem::new("remote: alice@host")
        .separator("═")                  // fill char, repeated to span the full width
        .color(Color::Yellow)
        .align(Alignment::Center)        // Left (default) / Center / Right
        .button("⟳")                     // right-aligned [⟳]
        .button("⋯"),                    // then [⋯]
    2,
);

// Route clicks through `handle_click_basic` to learn which button was hit:
if let Some((section, button)) = state.handle_click_basic(&mut list, col, row) {
    // a header button was clicked — the section is NOT toggled; run its action
} // otherwise it focused a row or toggled a section, like `handle_click`
```

## Custom rows

When `BasicItem` isn't enough, keep the widget but supply your own payload `T` and a render closure mapping each item to ratatui [`Text`]. You decide appearance; the widget still owns scroll, clipping, and placement:

```rust
use ratatui::text::Text;
use ratatui_sectioned_list::ItemKind;
use ratatui_sectioned_list::widget::SectionedListWidget;

let widget = SectionedListWidget::new(&list, |item, ctx| {
    // ctx: { row_idx, focused, collapsible, collapsed, width }
    match item.kind {
        ItemKind::Header => Text::raw(format!("== {} ==", item.data.title)),
        ItemKind::Row => {
            let marker = if ctx.focused { "> " } else { "  " };
            Text::raw(format!("{marker}{}", item.data.title))
        }
    }
});
frame.render_stateful_widget(widget, area, &mut state);
```

## Pure geometry core (no ratatui)

With `default-features = false` you get only the `u16` layout engine — no ratatui types, no dependencies. You drive the render loop yourself off `visible_items`:

```rust
use ratatui_sectioned_list::{ItemKind, SectionedList};

let mut list = SectionedList::new();
list.push_header("local", 1);
list.push_row("session-a", 3);

let focused = 1usize;
let viewport_height = 10u16;
let scroll = list.scroll_offset(Some(focused), viewport_height);

for v in list.visible_items(scroll, viewport_height) {
    // v.viewport_y     — top of this item inside the viewport
    // v.visible_height — height after top/bottom clipping
    // v.item_y_offset  — first visible line inside this item
    // v.item.data      — your &T
    // v.row_idx        — Some(n) for rows, None for headers
    match v.item.kind {
        ItemKind::Header => { /* draw header */ }
        ItemKind::Row    => { /* draw row, focus = v.row_idx == Some(focused) */ }
    }
}

// Mouse click at viewport row 4:
if let Some(row_idx) = list.row_at_y(4, scroll) {
    // focus row_idx
}
```

This is exactly what the widget does internally — the widget is a thin shell over it.

## Collapsible sections

Opt in with `set_collapsible(true)` (off by default — when off, the layout is unchanged). A header can then be collapsed, hiding its rows. With the widget, `SectionedListState::toggle_focused_section` and `handle_click` drive this for you. With the core directly:

```rust
list.set_collapsible(true);

// Click handler: a divider toggles its section, anything else focuses a row.
if let Some(section) = list.header_at_y(viewport_y, scroll) {
    list.toggle_section(section);
} else if let Some(row_idx) = list.row_at_y(viewport_y, scroll) {
    focused = row_idx;
}
```

Collapsing a section sets its rows to an effective height of 0 — they vanish
from `visible_items`, `row_at_y`, `total_height`, and scrolling, while keeping
their **stable global indices**. `is_row_hidden(row_idx)` lets keyboard / wheel
navigation step over them. The renderer reads `is_collapsible()` + the header's
`collapsed` flag to draw the ▾/▸ indicator (the `basic` preset does this already).

## Feature flags

| Feature | Default | What you get |
|---|---|---|
| `ratatui` | **on** | The `widget` module: `SectionedListWidget`, `SectionedListState`, the `BasicItem` preset, and `basic_style`. Pulls in `ratatui`. |
| *(none)* | — | `default-features = false`: the pure `u16` geometry core, zero dependencies, no ratatui types. |

## API

### Core (`SectionedList<T>`, always available)

| Method | Purpose |
|---|---|
| `push_header(data, height)` | Add a non-focusable header. |
| `push_row(data, height)` | Add a focusable row. |
| `push_header_auto(data)` | Add a header at the default height of 1 row. |
| `push_header_margin(data, margin_top)` | Add a header with `margin_top` blank rows above it — counted in layout but inert to hit-testing (section spacing). |
| `push_row_auto(data)` | Add a row sized to `data.row_height()` (requires `T: RowHeight`). |
| `set_row_height(global_idx, h)` | Resize an existing row. Returns `false` if out of range. |
| `total_height()` | Sum of all (effective) item heights. |
| `row_count()` | Number of focusable rows (headers skipped). |
| `row_y(row_idx)` | `(y_top, y_bottom)` of the nth row, or `None`. |
| `locate_row(global_idx)` | `RowLocation { section, row_in_section }` — section-scoped lookup. |
| `scroll_offset(focused, viewport_height)` | Minimum offset to keep focus visible. |
| `row_at_y(viewport_y, scroll_offset)` | Hit-test → global row index, or `None`. |
| `header_at_y(viewport_y, scroll_offset)` | Hit-test → section index of the divider at `y`, or `None`. |
| `set_collapsible(bool)` / `is_collapsible()` | Toggle/query the collapse feature. Default off. |
| `toggle_section(i)` / `set_collapsed(i, bool)` / `is_collapsed(i)` | Manage a section's collapsed state (`i` = 0-based header order). |
| `is_row_hidden(row_idx)` | Whether a row is hidden by a collapsed section — skip these when moving focus. |
| **`visible_items(scroll, viewport_height)`** | **The high-level rendering iterator: viewport-clipped `Visible<T>` with `viewport_y`, `visible_height`, `item_y_offset`, `row_idx`.** |
| `iter_with_y()` | Low-level walk over all items with their top y offset. |
| `items()` | Borrow the underlying item slice. |
| `layout_divider(spec)` | Compute label/rule widths, badge range, and right-side action ranges for a one-line divider. |

### Widget (`ratatui` feature)

| Item | Purpose |
|---|---|
| `SectionedListWidget::new(&list, render_fn)` | `StatefulWidget` over any `T`; `render_fn(&Item<T>, ItemContext) -> Text` decides appearance. |
| `SectionedListWidget::basic(&list)` | Shortcut for a `SectionedList<BasicItem>` using the `basic_style` preset, with a default focus highlight. |
| `.highlight_style(Style)` | Style painted across the focused row's full cell (e.g. a background bar), like ratatui's `List::highlight_style`. |
| `SectionedListState` | Focus + last-render geometry. `move_focus`, `toggle_focused_section`, `handle_click`, `ensure_focus_visible`, `focused`, `scroll_offset`. |
| `SectionedListState::handle_click_basic(&mut list, col, row)` | Like `handle_click` for `BasicItem` lists, but returns `Some((section, button))` when a header button is clicked (without toggling). |
| `BasicItem` | Built-in payload: `title`, dim `lines`, header `separator`, accent `color`, `align`, and right-side `buttons`, with chained setters. Implements `RowHeight`. |
| `basic_style` | Default render preset for `BasicItem`: full-width aligned divider + `[icon]` buttons. |

The generic `T` is your row payload — a label, a struct, anything. The core doesn't inspect it; implement `RowHeight` on it to enable `push_row_auto`.

## Design choices

- **Batteries included, core stays clean.** The default build is for ratatui users; the agnostic `u16` core is the same code, just reachable on its own via `default-features = false` (zero dependencies, no ratatui types).
- **Stateless core, optional state shell.** `SectionedList` itself holds no focus or scroll — you pass them in. The widget adds `SectionedListState` only as a convenience. Collapse is the exception: it changes layout heights, so the list owns it.
- **Focus indexes count rows only.** Headers are invisible to focus. Row indexes stay stable across collapse and as long as you don't reorder.
- **Anchor-bottom scrolling.** When focus doesn't fit from the top, the focused row's bottom edge aligns to the viewport's bottom edge — the smallest scroll that contains the focused row.

## Examples

```sh
cargo run --example basic            # non-interactive API tour (no TUI)
cargo run --example preset           # batteries-included BasicItem path
cargo run --example ratatui_sidebar  # the same, plus row resize, header buttons, and a live state readout
```

The two interactive demos are full-screen TUIs: `j`/`k` (or arrows) and the wheel
move focus, `enter`/space collapses the focused section, left-click focuses a row,
toggles a divider, or hits a header button, and `q`/`Esc` quits. `ratatui_sidebar`
also has `+`/`-` to resize the focused row and a right pane that reads
`scroll_offset` / `row_y` and the last button clicked.

## Status

Pre-1.0. The surface is small and intentionally minimal; API changes are possible if real-world use uncovers gaps.

## License

Apache-2.0 — see [LICENSE](./LICENSE).
