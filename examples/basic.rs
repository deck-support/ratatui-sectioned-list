//! Non-interactive API tour. Run with:
//!
//! ```sh
//! cargo run --example basic
//! ```
//!
//! Walks through every public method of `SectionedList` and prints the
//! result, so you can see the data structure without a TUI.

use ratatui_sectioned_list::{ItemKind, SectionedList};

fn main() {
    // Build a list mixing headers and variable-height rows.
    // `data` is `&str` here — it can be any type.
    let mut list = SectionedList::new();
    list.push_header("local", 1);
    list.push_row("session-a", 2);
    list.push_row("session-b", 3);
    list.push_header("remote: alice@host", 1);
    list.push_row("session-c", 2);
    list.push_row("session-d", 2);

    println!("Layout (total height = {}):", list.total_height());
    for (y, item) in list.iter_with_y() {
        let tag = match item.kind {
            ItemKind::Header => "HEADER",
            ItemKind::Row => "  row ",
        };
        println!(
            "  y={:>2}..{:<2}  {}  {}",
            y,
            y + item.height,
            tag,
            item.data,
        );
    }

    println!("\nRow count (headers skipped): {}", list.row_count());
    for i in 0..list.row_count() {
        println!("  row {} → y={:?}", i, list.row_y(i));
    }

    let viewport = 6u16;
    println!("\nScroll offsets for viewport_height = {viewport}:");
    for i in 0..list.row_count() {
        let off = list.scroll_offset(Some(i), viewport);
        println!("  focused row {i} → scroll_offset = {off}");
    }

    println!("\nHit-test at scroll_offset = 0:");
    for vy in 0..list.total_height() {
        let hit = list.row_at_y(vy, 0);
        let label = match hit {
            Some(i) => format!("row {i}"),
            None => "(header or empty)".to_string(),
        };
        println!("  viewport_y={vy} → {label}");
    }

    println!("\nHit-test at scroll_offset = 3 (sidebar scrolled down):");
    for vy in 0..viewport {
        let hit = list.row_at_y(vy, 3);
        let label = match hit {
            Some(i) => format!("row {i}"),
            None => "(header or past end)".to_string(),
        };
        println!("  viewport_y={vy} → {label}");
    }
}
