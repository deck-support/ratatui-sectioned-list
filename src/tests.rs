use super::*;

#[test]
fn push_header_auto_defaults_to_height_one() {
    let mut list = SectionedList::new();
    list.push_header_auto("h");
    list.push_row("a", 2);
    assert_eq!(list.row_y(0), Some((1, 3)));
    assert_eq!(list.total_height(), 3);
}

struct Sized(u16);
impl RowHeight for Sized {
    fn row_height(&self) -> u16 {
        self.0
    }
}

#[test]
fn push_row_auto_uses_row_height() {
    let mut list = SectionedList::new();
    list.push_row_auto(Sized(3));
    list.push_row_auto(Sized(2));
    assert_eq!(list.row_y(0), Some((0, 3)));
    assert_eq!(list.row_y(1), Some((3, 5)));
    assert_eq!(list.total_height(), 5);
}

#[test]
fn push_header_margin_excludes_lead_rows_from_hit_test() {
    let mut list = SectionedList::new();
    list.push_header_margin("h", 2); // height 3: rows 0,1 margin; row 2 bar
    list.push_row("a", 1); // y=3
    list.set_collapsible(true);
    assert_eq!(list.total_height(), 4);
    // The margin rows belong to no clickable target.
    assert_eq!(list.header_at_y(0, 0), None);
    assert_eq!(list.header_at_y(1, 0), None);
    assert_eq!(list.row_at_y(0, 0), None);
    assert_eq!(list.row_at_y(1, 0), None);
    // Only the bar row toggles the section.
    assert_eq!(list.header_at_y(2, 0), Some(0));
    // The row below is unaffected.
    assert_eq!(list.row_at_y(3, 0), Some(0));
}

#[test]
fn divider_layout_places_single_action_after_rule() {
    let layout = layout_divider(DividerLayoutSpec {
        width: 24,
        leading_width: 1,
        chevron_width: 2,
        spacer_width: 1,
        gap_width: 1,
        label_width: 6,
        badge_width: None,
        action_widths: vec![3],
    });
    assert_eq!(layout.label_width, 6);
    assert_eq!(layout.rule_width, 10);
    assert_eq!(layout.badge, None);
    assert_eq!(layout.actions, vec![21..24]);
}

#[test]
fn divider_layout_badge_eats_rule_not_action_positions() {
    let without = layout_divider(DividerLayoutSpec {
        width: 60,
        leading_width: 1,
        chevron_width: 2,
        spacer_width: 1,
        gap_width: 1,
        label_width: 2,
        badge_width: None,
        action_widths: vec![3, 3],
    });
    let with = layout_divider(DividerLayoutSpec {
        badge_width: Some(2),
        ..DividerLayoutSpec {
            width: 60,
            leading_width: 1,
            chevron_width: 2,
            spacer_width: 1,
            gap_width: 1,
            label_width: 2,
            badge_width: None,
            action_widths: vec![3, 3],
        }
    });
    assert_eq!(with.actions, without.actions);
    assert_eq!(with.badge, Some(50..52));
    assert_eq!(with.rule_width + 3, without.rule_width);
}

#[test]
fn divider_layout_suppresses_badge_when_it_would_crowd_label() {
    let layout = layout_divider(DividerLayoutSpec {
        width: 14,
        leading_width: 1,
        chevron_width: 2,
        spacer_width: 1,
        gap_width: 1,
        label_width: 2,
        badge_width: Some(3),
        action_widths: vec![3, 3],
    });
    assert_eq!(layout.badge, None);
    assert_eq!(layout.actions, vec![7..10, 11..14]);
    assert_eq!(layout.label_width, 2);
}

#[test]
fn empty_list_has_zero_total_height() {
    let list: SectionedList<&str> = SectionedList::new();
    assert_eq!(list.total_height(), 0);
}

#[test]
fn total_height_sums_header_and_row_heights() {
    let mut list = SectionedList::new();
    list.push_header("group", 1);
    list.push_row("a", 3);
    list.push_row("b", 2);
    assert_eq!(list.total_height(), 6);
}

#[test]
fn row_count_skips_headers() {
    let mut list = SectionedList::new();
    list.push_header("h1", 1);
    list.push_row("a", 1);
    list.push_row("b", 1);
    list.push_header("h2", 1);
    list.push_row("c", 1);
    assert_eq!(list.row_count(), 3);
}

#[test]
fn row_y_returns_top_and_bottom_for_nth_row() {
    let mut list = SectionedList::new();
    list.push_header("h", 1);
    list.push_row("a", 3);
    list.push_row("b", 2);
    // Row 0 ("a") starts at y=1 (after header), ends at y=4.
    assert_eq!(list.row_y(0), Some((1, 4)));
    // Row 1 ("b") starts at y=4, ends at y=6.
    assert_eq!(list.row_y(1), Some((4, 6)));
    // Out of range.
    assert_eq!(list.row_y(2), None);
}

#[test]
fn row_y_with_interleaved_headers() {
    let mut list = SectionedList::new();
    list.push_header("h1", 1);
    list.push_row("a", 2); // y 1..3
    list.push_header("h2", 1); // y 3..4
    list.push_row("b", 2); // y 4..6
    assert_eq!(list.row_y(0), Some((1, 3)));
    assert_eq!(list.row_y(1), Some((4, 6)));
}

#[test]
fn scroll_offset_is_zero_when_focus_unset() {
    let mut list = SectionedList::new();
    list.push_row("a", 3);
    assert_eq!(list.scroll_offset(None, 10), 0);
}

#[test]
fn scroll_offset_is_zero_when_focused_row_already_fits() {
    let mut list = SectionedList::new();
    list.push_header("h", 1);
    list.push_row("a", 3);
    list.push_row("b", 2);
    // Row 1 ends at y=6, viewport height 10 — fits.
    assert_eq!(list.scroll_offset(Some(1), 10), 0);
}

#[test]
fn scroll_offset_aligns_focused_row_bottom_to_viewport_bottom() {
    let mut list = SectionedList::new();
    list.push_header("h", 1);
    list.push_row("a", 3);
    list.push_row("b", 2);
    // Row 1 ends at y=6. Viewport height 5 → offset = 6 - 5 = 1.
    assert_eq!(list.scroll_offset(Some(1), 5), 1);
}

#[test]
fn scroll_offset_returns_zero_for_out_of_range_focus() {
    let mut list = SectionedList::new();
    list.push_row("a", 3);
    assert_eq!(list.scroll_offset(Some(99), 5), 0);
}

#[test]
fn scroll_offset_zero_viewport_returns_zero() {
    let mut list = SectionedList::new();
    list.push_row("a", 3);
    assert_eq!(list.scroll_offset(Some(0), 0), 0);
}

#[test]
fn row_at_y_returns_none_for_header() {
    let mut list = SectionedList::new();
    list.push_header("h", 1);
    list.push_row("a", 3);
    // viewport_y=0 lands on the header.
    assert_eq!(list.row_at_y(0, 0), None);
}

#[test]
fn row_at_y_returns_row_index_inside_row_span() {
    let mut list = SectionedList::new();
    list.push_header("h", 1);
    list.push_row("a", 3); // layout y=1..4
    list.push_row("b", 2); // layout y=4..6
                           // No scroll: viewport_y=1 → row 0 (top of "a").
    assert_eq!(list.row_at_y(1, 0), Some(0));
    // Bottom of "a" still in row 0.
    assert_eq!(list.row_at_y(3, 0), Some(0));
    // Top of "b" → row 1.
    assert_eq!(list.row_at_y(4, 0), Some(1));
    assert_eq!(list.row_at_y(5, 0), Some(1));
}

#[test]
fn row_at_y_returns_none_past_end() {
    let mut list = SectionedList::new();
    list.push_row("a", 2);
    // List ends at layout y=2. Anything >= 2 is past end.
    assert_eq!(list.row_at_y(2, 0), None);
    assert_eq!(list.row_at_y(100, 0), None);
}

#[test]
fn row_at_y_respects_scroll_offset() {
    let mut list = SectionedList::new();
    list.push_header("h", 1); // layout y=0
    list.push_row("a", 3); // layout y=1..4
    list.push_row("b", 2); // layout y=4..6
                           // Scroll offset 1: viewport top maps to layout y=1.
                           // viewport_y=0 → layout y=1 → row 0 ("a").
    assert_eq!(list.row_at_y(0, 1), Some(0));
    // viewport_y=3 → layout y=4 → row 1 ("b").
    assert_eq!(list.row_at_y(3, 1), Some(1));
    // viewport_y=4 → layout y=5 → still row 1.
    assert_eq!(list.row_at_y(4, 1), Some(1));
    // viewport_y=5 → layout y=6 → past end.
    assert_eq!(list.row_at_y(5, 1), None);
}

#[test]
fn row_drag_tracks_variable_height_rows_and_finishes_once() {
    let mut list = SectionedList::new();
    list.push_header("h", 1);
    list.push_row("a", 2);
    list.push_row("b", 3);

    let mut drag = RowDragState::new();
    assert_eq!(drag.begin(&list, 1, 0), Some(0));
    assert!(drag.is_active());
    assert_eq!(drag.update(&list, 4, 0), Some(1));
    assert_eq!(drag.target(), Some(1));
    assert_eq!(drag.finish(), Some(RowMove { from: 0, to: 1 }));
    assert!(!drag.is_active());
    assert_eq!(drag.finish(), None);
}

#[test]
fn row_drag_preserves_click_and_last_target_over_non_rows() {
    let mut list = SectionedList::new();
    list.push_header("first", 1);
    list.push_row("a", 1);
    list.push_header("second", 1);
    list.push_row("b", 1);

    let mut drag = RowDragState::new();
    assert_eq!(drag.begin(&list, 1, 0), Some(0));
    assert_eq!(drag.update(&list, 2, 0), Some(0)); // header
    assert_eq!(drag.update(&list, 99, 0), Some(0)); // past the list
    assert_eq!(drag.finish(), Some(RowMove { from: 0, to: 0 }));
}

#[test]
fn row_drag_uses_scroll_and_rejects_hidden_rows() {
    let mut list = SectionedList::new();
    list.set_collapsible(true);
    list.push_header("hidden", 1);
    list.push_row("a", 1);
    list.push_header("visible", 1);
    list.push_row("b", 2);
    assert!(list.set_collapsed(0, true));

    let mut drag = RowDragState::new();
    // With the first section collapsed, viewport row 1 is global row 1 (b).
    assert_eq!(drag.begin(&list, 1, 1), Some(1));
    assert_eq!(drag.source(), Some(1));
    drag.cancel();
    assert!(!drag.is_active());

    // A header cannot start a drag and also clears stale state.
    assert_eq!(drag.begin(&list, 0, 0), None);
    assert_eq!(drag.finish(), None);
}

#[test]
fn locate_row_returns_none_for_out_of_range() {
    let mut list = SectionedList::new();
    list.push_header("h", 1);
    list.push_row("a", 1);
    assert_eq!(list.locate_row(1), None);
    assert_eq!(list.locate_row(99), None);
}

#[test]
fn locate_row_assigns_rows_to_their_preceding_header() {
    let mut list = SectionedList::new();
    list.push_header("h0", 1);
    list.push_row("a", 1);
    list.push_row("b", 1);
    list.push_header("h1", 1);
    list.push_row("c", 1);
    assert_eq!(
        list.locate_row(0),
        Some(RowLocation {
            section: Some(0),
            row_in_section: 0
        })
    );
    assert_eq!(
        list.locate_row(1),
        Some(RowLocation {
            section: Some(0),
            row_in_section: 1
        })
    );
    assert_eq!(
        list.locate_row(2),
        Some(RowLocation {
            section: Some(1),
            row_in_section: 0
        })
    );
}

#[test]
fn locate_row_returns_none_section_when_row_precedes_any_header() {
    let mut list = SectionedList::new();
    list.push_row("a", 1);
    list.push_row("b", 1);
    list.push_header("h", 1);
    list.push_row("c", 1);
    assert_eq!(
        list.locate_row(0),
        Some(RowLocation {
            section: None,
            row_in_section: 0
        })
    );
    assert_eq!(
        list.locate_row(1),
        Some(RowLocation {
            section: None,
            row_in_section: 1
        })
    );
    assert_eq!(
        list.locate_row(2),
        Some(RowLocation {
            section: Some(0),
            row_in_section: 0
        })
    );
}

#[test]
fn locate_row_handles_consecutive_headers() {
    // First header has zero rows; second header is section 1.
    let mut list = SectionedList::new();
    list.push_header("h0", 1);
    list.push_header("h1", 1);
    list.push_row("a", 1);
    assert_eq!(
        list.locate_row(0),
        Some(RowLocation {
            section: Some(1),
            row_in_section: 0
        })
    );
}

#[test]
fn set_row_height_updates_the_targeted_row() {
    let mut list = SectionedList::new();
    list.push_header("h", 1);
    list.push_row("a", 2);
    list.push_row("b", 3);
    assert!(list.set_row_height(1, 5));
    // Row 0 unchanged at y 1..3; row 1 grew from h=3 to h=5.
    assert_eq!(list.row_y(0), Some((1, 3)));
    assert_eq!(list.row_y(1), Some((3, 8)));
    assert_eq!(list.total_height(), 8);
}

#[test]
fn set_row_height_returns_false_for_out_of_range() {
    let mut list = SectionedList::new();
    list.push_row("a", 2);
    assert!(!list.set_row_height(99, 5));
    // Untouched.
    assert_eq!(list.row_y(0), Some((0, 2)));
}

#[test]
fn set_row_height_does_not_touch_headers() {
    // Global index 0 must address the first ROW, never a header
    // that happens to come earlier in the item list.
    let mut list = SectionedList::new();
    list.push_header("h", 1);
    list.push_row("a", 2);
    assert!(list.set_row_height(0, 4));
    // Header still h=1.
    let kinds: Vec<(ItemKind, u16)> = list.items().iter().map(|it| (it.kind, it.height)).collect();
    assert_eq!(kinds, vec![(ItemKind::Header, 1), (ItemKind::Row, 4)]);
}

#[test]
fn set_row_height_allows_zero() {
    let mut list = SectionedList::new();
    list.push_row("a", 3);
    assert!(list.set_row_height(0, 0));
    assert_eq!(list.row_y(0), Some((0, 0)));
    assert_eq!(list.total_height(), 0);
}

#[test]
fn visible_items_yields_nothing_when_empty() {
    let list: SectionedList<&str> = SectionedList::new();
    let v: Vec<_> = list.visible_items(0, 10).collect();
    assert!(v.is_empty());
}

#[test]
fn visible_items_yields_nothing_when_viewport_height_is_zero() {
    let mut list = SectionedList::new();
    list.push_row("a", 3);
    let v: Vec<_> = list.visible_items(0, 0).collect();
    assert!(v.is_empty());
}

#[test]
fn visible_items_yields_all_when_viewport_fits_everything() {
    let mut list = SectionedList::new();
    list.push_header("h", 1);
    list.push_row("a", 2);
    list.push_row("b", 3);
    let v: Vec<_> = list.visible_items(0, 10).collect();
    assert_eq!(v.len(), 3);
    assert_eq!(
        (v[0].viewport_y, v[0].visible_height, v[0].row_idx),
        (0, 1, None)
    );
    assert_eq!(
        (v[1].viewport_y, v[1].visible_height, v[1].row_idx),
        (1, 2, Some(0))
    );
    assert_eq!(
        (v[2].viewport_y, v[2].visible_height, v[2].row_idx),
        (3, 3, Some(1))
    );
    assert_eq!(v[0].item_y_offset, 0);
    assert_eq!(v[1].item_y_offset, 0);
    assert_eq!(v[2].item_y_offset, 0);
}

#[test]
fn visible_items_clips_item_at_bottom_edge() {
    let mut list = SectionedList::new();
    list.push_row("a", 5); // layout 0..5
    list.push_row("b", 5); // layout 5..10
                           // Viewport 7 tall: "a" fully visible (0..5), "b" clipped (5..7 → vh=2).
    let v: Vec<_> = list.visible_items(0, 7).collect();
    assert_eq!(v.len(), 2);
    assert_eq!((v[0].viewport_y, v[0].visible_height), (0, 5));
    assert_eq!((v[1].viewport_y, v[1].visible_height), (5, 2));
    assert_eq!(v[1].item_y_offset, 0);
}

#[test]
fn visible_items_clips_item_at_top_edge() {
    let mut list = SectionedList::new();
    list.push_row("a", 5); // 0..5
    list.push_row("b", 5); // 5..10
                           // scroll=2: viewport sees layout 2..12. "a" clipped from 2..5 (vh=3), "b" full (5..10 → viewport_y=3, vh=5).
    let v: Vec<_> = list.visible_items(2, 10).collect();
    assert_eq!(v.len(), 2);
    assert_eq!(
        (v[0].viewport_y, v[0].visible_height, v[0].row_idx),
        (0, 3, Some(0))
    );
    assert_eq!(v[0].item_y_offset, 2);
    assert_eq!(
        (v[1].viewport_y, v[1].visible_height, v[1].row_idx),
        (3, 5, Some(1))
    );
    assert_eq!(v[1].item_y_offset, 0);
}

#[test]
fn visible_item_maps_item_lines_to_viewport_y() {
    let mut list = SectionedList::new();
    list.push_row("a", 5); // 0..5
    let v: Vec<_> = list.visible_items(2, 2).collect();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].item_y_offset, 2);
    assert_eq!(v[0].visible_height, 2);
    assert_eq!(v[0].viewport_y_for_item_line(1), None);
    assert_eq!(v[0].viewport_y_for_item_line(2), Some(0));
    assert_eq!(v[0].viewport_y_for_item_line(3), Some(1));
    assert_eq!(v[0].viewport_y_for_item_line(4), None);
}

#[test]
fn visible_items_skips_items_entirely_above_but_preserves_row_idx() {
    let mut list = SectionedList::new();
    list.push_row("a", 3); // 0..3
    list.push_row("b", 3); // 3..6
    list.push_row("c", 3); // 6..9
                           // scroll=4: "a" gone, "b" clipped 4..6, "c" full 6..9.
    let v: Vec<_> = list.visible_items(4, 10).collect();
    assert_eq!(v.len(), 2);
    // Critical: "b" must still report row_idx=Some(1), not Some(0).
    assert_eq!(v[0].row_idx, Some(1));
    assert_eq!(v[0].visible_height, 2);
    assert_eq!(v[1].row_idx, Some(2));
}

#[test]
fn visible_items_stops_iteration_when_past_viewport_bottom() {
    let mut list = SectionedList::new();
    list.push_row("a", 3); // 0..3
    list.push_row("b", 3); // 3..6
    list.push_row("c", 3); // 6..9 — entirely below a 4-tall viewport
    let v: Vec<_> = list.visible_items(0, 4).collect();
    assert_eq!(v.len(), 2);
    assert_eq!(v[1].visible_height, 1); // "b" clipped
}

#[test]
fn visible_items_skips_zero_height_items_without_breaking_row_idx() {
    let mut list = SectionedList::new();
    list.push_row("a", 2);
    list.push_row("b", 0);
    list.push_row("c", 2);
    let v: Vec<_> = list.visible_items(0, 10).collect();
    assert_eq!(v.len(), 2);
    assert_eq!(v[0].row_idx, Some(0));
    // "b" is hidden but "c" must still be Some(2), not Some(1).
    assert_eq!(v[1].row_idx, Some(2));
}

#[test]
fn visible_items_yields_nothing_when_scrolled_past_end() {
    let mut list = SectionedList::new();
    list.push_row("a", 3);
    // scroll=100 way past total_height=3.
    let v: Vec<_> = list.visible_items(100, 10).collect();
    assert!(v.is_empty());
}

#[test]
fn visible_items_item_starting_at_viewport_bottom_is_not_yielded() {
    let mut list = SectionedList::new();
    list.push_row("a", 5); // 0..5
    list.push_row("b", 5); // 5..10
                           // Viewport height 5 → covers 0..5. "b" starts exactly at the bottom edge — invisible.
    let v: Vec<_> = list.visible_items(0, 5).collect();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].row_idx, Some(0));
}

#[test]
fn iter_with_y_yields_top_offsets_in_order() {
    let mut list = SectionedList::new();
    list.push_header("group", 1);
    list.push_row("a", 3);
    list.push_row("b", 2);
    let observed: Vec<(u16, ItemKind, &str)> = list
        .iter_with_y()
        .map(|(y, it)| (y, it.kind, it.data))
        .collect();
    assert_eq!(
        observed,
        vec![
            (0, ItemKind::Header, "group"),
            (1, ItemKind::Row, "a"),
            (4, ItemKind::Row, "b"),
        ]
    );
}

// ----- collapsible sections -----

// Layout with no collapse:
//   h0 0..1, a 1..3, b 3..6, h1 6..7, c 7..9, d 9..11. total = 11.
fn collapsible_fixture() -> SectionedList<&'static str> {
    let mut list = SectionedList::new();
    list.push_header("h0", 1);
    list.push_row("a", 2);
    list.push_row("b", 3);
    list.push_header("h1", 1);
    list.push_row("c", 2);
    list.push_row("d", 2);
    list
}

#[test]
fn collapsible_defaults_to_false() {
    let list: SectionedList<&str> = SectionedList::new();
    assert!(!list.is_collapsible());
}

#[test]
fn collapse_has_no_layout_effect_until_collapsible_enabled() {
    let mut list = collapsible_fixture();
    // The stored collapsed flag is set regardless of the master switch.
    assert!(list.set_collapsed(0, true));
    assert!(list.is_collapsed(0));
    // With collapsible off, layout is unchanged and nothing is hidden.
    assert_eq!(list.total_height(), 11);
    assert!(!list.is_row_hidden(0));
    // Flipping the master switch on makes the stored collapse take effect.
    list.set_collapsible(true);
    assert_eq!(list.total_height(), 6);
    assert!(list.is_row_hidden(0));
    assert!(list.is_row_hidden(1));
}

#[test]
fn collapsing_a_section_hides_its_rows_but_keeps_the_header() {
    let mut list = collapsible_fixture();
    list.set_collapsible(true);
    assert!(list.toggle_section(0));
    // h0(1) + h1(1) + c(2) + d(2) = 6; rows a,b contribute 0.
    assert_eq!(list.total_height(), 6);
    let v: Vec<_> = list.visible_items(0, 20).collect();
    // h0, h1, c, d — a and b are skipped.
    assert_eq!(v.len(), 4);
    assert_eq!((v[0].row_idx, v[0].viewport_y), (None, 0)); // h0
    assert_eq!((v[1].row_idx, v[1].viewport_y), (None, 1)); // h1
    assert_eq!((v[2].row_idx, v[2].viewport_y), (Some(2), 2)); // c keeps index 2
    assert_eq!((v[3].row_idx, v[3].viewport_y), (Some(3), 4)); // d
}

#[test]
fn row_indices_stay_stable_across_collapse() {
    let mut list = collapsible_fixture();
    list.set_collapsible(true);
    list.set_collapsed(0, true);
    let v: Vec<_> = list.visible_items(0, 20).collect();
    let first_row = v.iter().find(|x| x.row_idx.is_some()).unwrap();
    // "c" is the first visible row but keeps its global index 2.
    assert_eq!(first_row.row_idx, Some(2));
}

#[test]
fn row_at_y_skips_collapsed_rows() {
    let mut list = collapsible_fixture();
    list.set_collapsible(true);
    list.set_collapsed(0, true);
    // Layout: h0 0..1, (a,b hidden), h1 1..2, c 2..4, d 4..6.
    assert_eq!(list.row_at_y(0, 0), None); // h0
    assert_eq!(list.row_at_y(1, 0), None); // h1
    assert_eq!(list.row_at_y(2, 0), Some(2)); // c
    assert_eq!(list.row_at_y(3, 0), Some(2));
    assert_eq!(list.row_at_y(4, 0), Some(3)); // d
    assert_eq!(list.row_at_y(5, 0), Some(3));
    assert_eq!(list.row_at_y(6, 0), None); // past end
}

#[test]
fn header_at_y_returns_section_index() {
    let mut list = collapsible_fixture();
    list.set_collapsible(true);
    // No collapse yet: h0 at y0, h1 at y6.
    assert_eq!(list.header_at_y(0, 0), Some(0));
    assert_eq!(list.header_at_y(6, 0), Some(1));
    // Row positions are not headers.
    assert_eq!(list.header_at_y(1, 0), None);
    assert_eq!(list.header_at_y(3, 0), None);
    // Past the end.
    assert_eq!(list.header_at_y(50, 0), None);
}

#[test]
fn header_at_y_tracks_collapse_shifted_positions() {
    let mut list = collapsible_fixture();
    list.set_collapsible(true);
    list.set_collapsed(0, true);
    // h1 moves up to y1 once section 0 collapses.
    assert_eq!(list.header_at_y(0, 0), Some(0));
    assert_eq!(list.header_at_y(1, 0), Some(1));
}

#[test]
fn row_y_and_scroll_offset_honor_collapse() {
    let mut list = collapsible_fixture();
    list.set_collapsible(true);
    list.set_collapsed(0, true);
    // Row d (global 3) sits at y4..6 once section 0 collapses.
    assert_eq!(list.row_y(3), Some((4, 6)));
    // Hidden row a (global 0) is zero-height at the collapse point.
    assert_eq!(list.row_y(0), Some((1, 1)));
    // Viewport height 4 → offset aligns d's bottom (6) to the bottom edge.
    assert_eq!(list.scroll_offset(Some(3), 4), 2);
}

#[test]
fn is_row_hidden_reflects_section_collapse() {
    let mut list = collapsible_fixture();
    list.set_collapsible(true);
    list.set_collapsed(0, true);
    assert!(list.is_row_hidden(0));
    assert!(list.is_row_hidden(1));
    assert!(!list.is_row_hidden(2));
    assert!(!list.is_row_hidden(3));
    // Out of range is never hidden.
    assert!(!list.is_row_hidden(99));
}

#[test]
fn toggle_and_set_collapsed_guard_out_of_range() {
    let mut list = collapsible_fixture();
    // Two headers → section indices 0 and 1 are valid.
    assert!(list.toggle_section(0));
    assert!(list.is_collapsed(0));
    assert!(list.toggle_section(0)); // toggles back
    assert!(!list.is_collapsed(0));
    assert!(!list.toggle_section(2));
    assert!(!list.set_collapsed(2, true));
    assert!(!list.is_collapsed(2));
}

#[test]
fn expanding_restores_layout() {
    let mut list = collapsible_fixture();
    list.set_collapsible(true);
    list.set_collapsed(0, true);
    assert_eq!(list.total_height(), 6);
    list.set_collapsed(0, false);
    assert_eq!(list.total_height(), 11);
    assert!(!list.is_row_hidden(0));
}

#[test]
fn collapsing_the_last_section_hides_its_trailing_rows() {
    let mut list = collapsible_fixture();
    list.set_collapsible(true);
    list.set_collapsed(1, true);
    // h0(1) + a(2) + b(3) + h1(1) = 7; c,d hidden.
    assert_eq!(list.total_height(), 7);
    let v: Vec<_> = list.visible_items(0, 20).collect();
    assert_eq!(v.len(), 4); // h0, a, b, h1
    assert_eq!(v.last().unwrap().row_idx, None); // last visible item is h1
}
