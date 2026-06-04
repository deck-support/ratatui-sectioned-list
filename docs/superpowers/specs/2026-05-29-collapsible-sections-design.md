# Collapsible sections — design

Date: 2026-05-29
Status: approved (core forks confirmed via question prompt)

## Goal

Add an opt-in `collapsible` flag to `SectionedList`. Default `false`. When
`true`, a caller can click a section divider (header) to collapse/expand the
section, and the divider shows an indicator of its state. When a section is
collapsed, keyboard (`j`/`k`) and mouse-wheel selection must skip the now-hidden
rows.

## Confirmed design decisions

1. **The list owns collapse state.** Collapse changes layout heights, which the
   list already owns (`set_row_height` is the precedent). Layout method
   signatures stay unchanged — no threading a collapse set through every call.
2. **The crate exposes state; the caller renders the indicator.** The crate
   stays renderer-agnostic and never inspects `T`. The example demonstrates the
   real `▾`/`▸` rendering.

## Semantics

- `collapsible` defaults to `false`. While `false`, layout is **identical to
  today** — collapse state has zero effect, all existing tests pass unchanged.
- Collapse is keyed by **section index**: the 0-based header order, matching
  `RowLocation.section`.
- Collapsing a section sets the **effective height of its rows to `0`** (the
  header/divider stays full height and visible). This reuses the existing
  zero-height-skipping path, so **row indices remain stable** across collapse —
  a hidden row keeps its global index, it just stops rendering and stops being
  hit-testable.
- Effective height of a row = `0` when `collapsible && its_section.collapsed`,
  else its declared height. A single internal collapse-aware walk computes this;
  `total_height`, `row_y`, `scroll_offset`, `row_at_y`, `visible_items`, and the
  new `header_at_y` are all built on it.

## Public API

### `Item<T>`
- New public field `collapsed: bool` (meaningful only for headers; starts
  `false`). The renderer reads `v.item.collapsed` to draw the indicator.

> Minor breaking change: adding a public field breaks `Item { .. }` literal
> construction. Acceptable per pre-1.0 status; noted in README.

### `SectionedList<T>`
| Method | Purpose |
|---|---|
| `set_collapsible(bool)` / `is_collapsible() -> bool` | The master flag. Default false; `#[derive(Default)]` preserved. |
| `toggle_section(section_idx) -> bool` | Flip a section's collapsed state. `false` if out of range. |
| `set_collapsed(section_idx, bool) -> bool` | Set explicitly. `false` if out of range. |
| `is_collapsed(section_idx) -> bool` | Query. `false` for out-of-range. |
| `header_at_y(viewport_y, scroll) -> Option<usize>` | Hit-test → section index of the divider at `y`. Mirror of `row_at_y`. |
| `is_row_hidden(row_idx) -> bool` | True if the row sits in a collapsed (and collapsible) section — lets the caller skip hidden rows during `j`/`k` and wheel navigation. |

`row_at_y` keeps its signature; collapsed rows are zero-height so it already
returns `None` for them.

## Indicator (rendered by caller)

The crate exposes `item.collapsed` + `is_collapsible()`. The example draws the
divider with `▾` when expanded and `▸` when collapsed, only while collapsible.

## Navigation skipping (caller responsibility, demoed in example)

`is_row_hidden(row_idx)` lets the caller advance focus past hidden rows. The
example's `move_focus` loops in the step direction until it lands on a visible
row, and re-homes focus to a visible row if a collapse hides the focused row.

## Scope of change

- `src/lib.rs`: API additions + internal collapse-aware walk + unit tests:
  - flag-off ⇒ collapse has no effect (back-compat)
  - collapse hides a section's rows; header stays; `total_height` drops
  - row indices stay stable across collapse
  - `row_at_y` returns `None` inside a collapsed region; correct rows + shifted y after
  - `header_at_y` maps to the right section index, honoring collapse-shifted y; `None` on a row / empty space
  - `scroll_offset` honors a collapsed section above the focused row
  - `is_row_hidden` correctness
  - `toggle`/`set_collapsed`/`is_collapsed` out-of-range guards
  - state stored while disabled, applied when `collapsible` flips on
- `examples/ratatui_sidebar.rs`: enable `collapsible`, draw `▾/▸`, click divider
  to toggle, skip hidden rows in `j`/`k`.
- `README.md`: API table rows + a short "Collapsible sections" note + the
  `Item.collapsed` breaking-change note.
