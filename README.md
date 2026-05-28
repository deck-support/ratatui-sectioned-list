# ratatui-sectioned-list

Layout, focus, scroll, and hit-test primitives for a **sectioned list** — non-focusable headers interleaved with focusable, variable-height rows.

UI-framework agnostic: heights and offsets are plain `u16` values measured in terminal rows. You render. This crate just answers:

- How tall is the layout?
- Where does row N live?
- What's the smallest scroll offset that keeps row N visible?
- The user clicked at viewport `y` — which row did they hit?

![interactive ratatui demo](https://raw.githubusercontent.com/deck-support/ratatui-sectioned-list/main/SCR-20260529-bvkh.png)

## When you need this

You're using [ratatui](https://github.com/ratatui/ratatui) (or any TUI lib) and your list has:

- **Section headers** — visible labels that aren't focusable.
- **Variable-height rows** — some rows are 1 line, some are 3.
- **Mouse hit-testing** — clicks need to map back to row index.

[`tui-widget-list`](https://crates.io/crates/tui-widget-list) is the closest existing crate, but it doesn't model the header-row distinction or expose viewport-relative hit-testing. This crate fills that gap with a tiny, dependency-free API.

## Usage

```rust
use ratatui_sectioned_list::{ItemKind, SectionedList};

let mut list = SectionedList::new();
list.push_header("local", 1);
list.push_row("session-a", 3);
list.push_row("session-b", 3);
list.push_header("remote: alice@host", 1);
list.push_row("session-c", 3);

let focused: usize = 2;
let viewport_height: u16 = 10;
let scroll = list.scroll_offset(Some(focused), viewport_height);

// Render — clipping, scroll subtraction, and row indexing are folded
// into the iterator. You translate `Visible` into your renderer.
for v in list.visible_items(scroll, viewport_height) {
    // v.viewport_y         — top of this item inside the viewport
    // v.visible_height     — height after top/bottom clipping
    // v.item.data          — your &T
    // v.row_idx            — Some(n) for rows, None for headers
    let is_focused_row = v.row_idx == Some(focused);
    match v.item.kind {
        ItemKind::Header => { /* draw header */ }
        ItemKind::Row    => { /* draw row, possibly with focus styling */ }
    }
}

// Mouse click at viewport row 4:
if let Some(row_idx) = list.row_at_y(4, scroll) {
    // focus row_idx
}
```

See `examples/ratatui_sidebar.rs` for a complete interactive demo (keyboard nav, mouse click, dynamic resize).

## API

| Method | Purpose |
|---|---|
| `push_header(data, height)` | Add a non-focusable header. |
| `push_row(data, height)` | Add a focusable row. |
| `set_row_height(global_idx, h)` | Resize an existing row. Returns `false` if out of range. |
| `total_height()` | Sum of all item heights. |
| `row_count()` | Number of focusable rows (headers skipped). |
| `row_y(row_idx)` | `(y_top, y_bottom)` of the nth row, or `None`. |
| `locate_row(global_idx)` | `RowLocation { section, row_in_section }` — section-scoped lookup. |
| `scroll_offset(focused, viewport_height)` | Minimum offset to keep focus visible. |
| `row_at_y(viewport_y, scroll_offset)` | Hit-test → global row index, or `None`. |
| **`visible_items(scroll, viewport_height)`** | **The high-level rendering iterator: yields viewport-clipped `Visible<T>` entries with `viewport_y`, `visible_height`, `row_idx`.** |
| `iter_with_y()` | Low-level walk over all items with their top y offset. |
| `items()` | Borrow the underlying item slice. |

The generic `T` is your row payload — a label, a struct, anything. The crate doesn't inspect it.

## Design choices

- **No dependencies.** Not even `ratatui`. Heights are `u16`, offsets are `u16`. You bring your own renderer.
- **Stateless.** `SectionedList` doesn't hold focus or scroll state — pass them as parameters. Compose freely with whatever state-management style you already use.
- **Focus indexes count rows only.** Headers are invisible to focus. Row indexes are stable as long as you don't reorder.
- **Anchor-bottom scrolling.** When focus doesn't fit from the top, the focused row's bottom edge aligns to the viewport's bottom edge. This is the smallest scroll that contains the focused row.

## Examples

```sh
cargo run --example basic              # non-interactive API tour
cargo run --example ratatui_sidebar    # interactive ratatui demo with mouse + keyboard
```

The ratatui demo shows a sidebar of variable-height session cards grouped under host headers, with `j/k` (or arrow keys) navigation, mouse-click hit-testing, and a live readout of `scroll_offset` / `row_y` in the right pane.

## Status

Pre-1.0. The surface is small and intentionally minimal; API changes are possible if real-world use uncovers gaps.

## License

MIT OR Apache-2.0
