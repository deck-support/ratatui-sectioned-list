//! Layout, focus, scroll, and hit-test primitives for a sectioned list —
//! a list of non-focusable headers interleaved with focusable variable-height
//! rows.
//!
//! This crate is UI-framework agnostic: heights and offsets are plain `u16`
//! values measured in terminal rows. The caller renders.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Header,
    Row,
}

#[derive(Debug, Clone)]
pub struct Item<T> {
    pub kind: ItemKind,
    pub height: u16,
    pub data: T,
    /// Whether this section is collapsed. Meaningful only for headers
    /// (rows ignore it). When the list is `collapsible` and a header is
    /// collapsed, the rows belonging to that section are laid out with an
    /// effective height of 0 — hidden, but with their global indices intact.
    /// Read this in your renderer to draw a collapse/expand indicator.
    pub collapsed: bool,
}

/// One item yielded by [`SectionedList::visible_items`].
///
/// Geometry is already viewport-relative and clipped to the viewport:
/// `viewport_y` is the row's top inside the viewport, `visible_height`
/// is its height after clipping to the viewport edges. The caller
/// renders directly using these values — no further math required.
#[derive(Debug)]
pub struct Visible<'a, T> {
    pub item: &'a Item<T>,
    pub viewport_y: u16,
    pub visible_height: u16,
    /// `Some(row_idx)` for focusable rows; `None` for headers. The row
    /// index is the global one — counted across the whole list, with
    /// headers skipped, preserved across rows that are above the
    /// viewport or have height 0.
    pub row_idx: Option<usize>,
}

/// Iterator returned by [`SectionedList::visible_items`].
///
/// Built on top of [`SectionedList::walk`], so the effective-height /
/// collapse arithmetic lives in exactly one place; this iterator adds only
/// the viewport clipping, zero-height skipping, and row-index counting.
pub struct VisibleIter<'a, T> {
    walk: Box<dyn Iterator<Item = (u16, &'a Item<T>, u16)> + 'a>,
    scroll: u16,
    viewport_height: u16,
    row_idx_counter: usize,
    finished: bool,
}

impl<'a, T> Iterator for VisibleIter<'a, T> {
    type Item = Visible<'a, T>;

    fn next(&mut self) -> Option<Visible<'a, T>> {
        if self.finished || self.viewport_height == 0 {
            return None;
        }
        let view_top = self.scroll;
        let view_bottom = self.scroll.saturating_add(self.viewport_height);

        loop {
            let (item_top, item, eff_height) = self.walk.next()?;
            let item_bottom = item_top.saturating_add(eff_height);

            let row_idx = match item.kind {
                ItemKind::Row => {
                    let r = Some(self.row_idx_counter);
                    self.row_idx_counter += 1;
                    r
                }
                ItemKind::Header => None,
            };

            // Item starts at or below the viewport bottom — all remaining
            // items are also below, so stop iterating altogether.
            if item_top >= view_bottom {
                self.finished = true;
                return None;
            }
            if item_bottom <= view_top {
                continue;
            }
            if eff_height == 0 {
                continue;
            }

            let overlap_top = item_top.max(view_top);
            let overlap_bottom = item_bottom.min(view_bottom);
            let visible_height = overlap_bottom - overlap_top;
            let viewport_y = overlap_top - view_top;

            return Some(Visible {
                item,
                viewport_y,
                visible_height,
                row_idx,
            });
        }
    }
}

/// Where a focusable row sits in the section hierarchy.
///
/// `section` is the 0-based index of the most recent header before
/// the row, or `None` if the row appears before any header.
/// `row_in_section` is the 0-based position of the row inside its
/// containing section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowLocation {
    pub section: Option<usize>,
    pub row_in_section: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SectionedList<T> {
    items: Vec<Item<T>>,
    collapsible: bool,
}

impl<T> SectionedList<T> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            collapsible: false,
        }
    }

    pub fn push_header(&mut self, data: T, height: u16) {
        self.items.push(Item {
            kind: ItemKind::Header,
            height,
            data,
            collapsed: false,
        });
    }

    pub fn push_row(&mut self, data: T, height: u16) {
        self.items.push(Item {
            kind: ItemKind::Row,
            height,
            data,
            collapsed: false,
        });
    }

    pub fn items(&self) -> &[Item<T>] {
        &self.items
    }

    /// Enable or disable collapsing. When `false` (the default), collapse
    /// state is ignored entirely and the layout behaves as if every section
    /// were expanded. Stored per-section collapsed flags are preserved across
    /// toggling this switch.
    pub fn set_collapsible(&mut self, collapsible: bool) {
        self.collapsible = collapsible;
    }

    /// Whether collapsing is enabled. Use this in your renderer to decide
    /// whether to draw collapse/expand indicators on dividers.
    pub fn is_collapsible(&self) -> bool {
        self.collapsible
    }

    /// Set a section's collapsed state. `section_idx` is the 0-based header
    /// order (same numbering as [`RowLocation::section`]). Returns `false` if
    /// `section_idx` is out of range.
    ///
    /// The stored flag is updated regardless of [`is_collapsible`]; it only
    /// affects layout while collapsing is enabled.
    ///
    /// [`is_collapsible`]: SectionedList::is_collapsible
    pub fn set_collapsed(&mut self, section_idx: usize, collapsed: bool) -> bool {
        match self.header_item_index(section_idx) {
            Some(i) => {
                self.items[i].collapsed = collapsed;
                true
            }
            None => false,
        }
    }

    /// Flip a section's collapsed state. Returns `false` if `section_idx` is
    /// out of range.
    pub fn toggle_section(&mut self, section_idx: usize) -> bool {
        match self.header_item_index(section_idx) {
            Some(i) => {
                self.items[i].collapsed = !self.items[i].collapsed;
                true
            }
            None => false,
        }
    }

    /// Whether the section's stored collapsed flag is set. Returns `false`
    /// for an out-of-range `section_idx`. Independent of [`is_collapsible`] —
    /// this reports the stored flag, not whether the rows are currently hidden.
    ///
    /// [`is_collapsible`]: SectionedList::is_collapsible
    pub fn is_collapsed(&self, section_idx: usize) -> bool {
        self.header_item_index(section_idx)
            .map(|i| self.items[i].collapsed)
            .unwrap_or(false)
    }

    /// Whether the focusable row at `row_idx` is currently hidden by a
    /// collapsed section. Always `false` when collapsing is disabled or the
    /// index is out of range. Use this to skip hidden rows when moving focus
    /// with the keyboard or mouse wheel.
    pub fn is_row_hidden(&self, row_idx: usize) -> bool {
        if !self.collapsible {
            return false;
        }
        match self.locate_row(row_idx) {
            Some(RowLocation {
                section: Some(s), ..
            }) => self.is_collapsed(s),
            _ => false,
        }
    }

    /// Item-vector index of the nth header (0-based header order), or `None`.
    fn header_item_index(&self, section_idx: usize) -> Option<usize> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, it)| it.kind == ItemKind::Header)
            .nth(section_idx)
            .map(|(i, _)| i)
    }

    /// Walk every item in order, yielding `(top_y, item, effective_height)`.
    /// The effective height is `0` for rows inside a collapsed section (when
    /// collapsing is enabled); headers always keep their declared height. This
    /// is the single source of truth that all layout queries are built on.
    fn walk(&self) -> impl Iterator<Item = (u16, &Item<T>, u16)> {
        let collapsible = self.collapsible;
        let mut y: u16 = 0;
        let mut section_collapsed = false;
        self.items.iter().map(move |it| {
            let eff = match it.kind {
                ItemKind::Header => {
                    section_collapsed = collapsible && it.collapsed;
                    it.height
                }
                ItemKind::Row => {
                    if section_collapsed {
                        0
                    } else {
                        it.height
                    }
                }
            };
            let cur = y;
            y = y.saturating_add(eff);
            (cur, it, eff)
        })
    }

    /// Total layout height, honoring collapsed sections.
    pub fn total_height(&self) -> u16 {
        self.walk().map(|(_, _, eff)| eff).sum()
    }

    /// Number of focusable rows. Headers don't count.
    pub fn row_count(&self) -> usize {
        self.items
            .iter()
            .filter(|it| it.kind == ItemKind::Row)
            .count()
    }

    /// Locate the nth focusable row in the layout.
    /// Returns `(y_top, y_bottom)` — half-open `[top, bottom)`.
    /// `None` if `row_idx` is out of range.
    pub fn row_y(&self, row_idx: usize) -> Option<(u16, u16)> {
        self.walk()
            .filter(|(_, it, _)| it.kind == ItemKind::Row)
            .nth(row_idx)
            .map(|(y, _, eff)| (y, y.saturating_add(eff)))
    }

    /// Smallest scroll offset (in rows) that keeps the focused row visible
    /// inside a viewport of `viewport_height` rows. The focused row's
    /// bottom edge is aligned to the viewport's bottom edge when the row
    /// doesn't already fit from the top.
    ///
    /// Returns 0 when focus is unset, out of range, or the viewport has
    /// zero height.
    pub fn scroll_offset(&self, focused: Option<usize>, viewport_height: u16) -> u16 {
        let Some(idx) = focused else {
            return 0;
        };
        if viewport_height == 0 {
            return 0;
        }
        let Some((_, y_bottom)) = self.row_y(idx) else {
            return 0;
        };
        y_bottom.saturating_sub(viewport_height)
    }

    /// Hit-test: map a viewport-relative y coordinate to a focusable row
    /// index, given the current `scroll_offset`. Returns `None` if the
    /// coordinate lands on a header or past the end of the list.
    pub fn row_at_y(&self, viewport_y: u16, scroll_offset: u16) -> Option<usize> {
        let layout_y = viewport_y.checked_add(scroll_offset)?;
        let mut row_idx: usize = 0;
        for (top, it, eff) in self.walk() {
            let bottom = top.saturating_add(eff);
            if layout_y >= top && layout_y < bottom {
                return match it.kind {
                    ItemKind::Row => Some(row_idx),
                    ItemKind::Header => None,
                };
            }
            if it.kind == ItemKind::Row {
                row_idx += 1;
            }
        }
        None
    }

    /// Hit-test: map a viewport-relative y coordinate to a section index (the
    /// 0-based header order, matching [`RowLocation::section`]), given the
    /// current `scroll_offset`. Returns `None` if the coordinate lands on a
    /// row or past the end of the list.
    ///
    /// This is the mirror of [`row_at_y`] for dividers: a click handler tries
    /// `header_at_y` first and, if it returns `Some(section)` while collapsing
    /// is enabled, calls [`toggle_section`].
    ///
    /// [`row_at_y`]: SectionedList::row_at_y
    /// [`toggle_section`]: SectionedList::toggle_section
    pub fn header_at_y(&self, viewport_y: u16, scroll_offset: u16) -> Option<usize> {
        let layout_y = viewport_y.checked_add(scroll_offset)?;
        let mut section_idx: usize = 0;
        for (top, it, eff) in self.walk() {
            if it.kind == ItemKind::Header {
                let bottom = top.saturating_add(eff);
                if layout_y >= top && layout_y < bottom {
                    return Some(section_idx);
                }
                section_idx += 1;
            }
        }
        None
    }

    /// Resolve a global row index to its (section, row-in-section) location.
    ///
    /// The global row index counts only focusable rows (headers skipped).
    /// `section` is `None` if the row appears before any header; otherwise
    /// it's the 0-based header index. `row_in_section` resets to 0 at each
    /// header.
    pub fn locate_row(&self, global_idx: usize) -> Option<RowLocation> {
        let mut section: Option<usize> = None;
        let mut row_in_section: usize = 0;
        let mut rows_seen: usize = 0;
        for item in &self.items {
            match item.kind {
                ItemKind::Header => {
                    section = Some(section.map_or(0, |s| s + 1));
                    row_in_section = 0;
                }
                ItemKind::Row => {
                    if rows_seen == global_idx {
                        return Some(RowLocation {
                            section,
                            row_in_section,
                        });
                    }
                    rows_seen += 1;
                    row_in_section += 1;
                }
            }
        }
        None
    }

    /// Change the height of the nth focusable row (headers don't count).
    /// Returns `true` if `global_idx` was valid, `false` otherwise.
    pub fn set_row_height(&mut self, global_idx: usize, height: u16) -> bool {
        let mut rows_seen = 0;
        for item in &mut self.items {
            if item.kind == ItemKind::Row {
                if rows_seen == global_idx {
                    item.height = height;
                    return true;
                }
                rows_seen += 1;
            }
        }
        false
    }

    /// Walk the items whose layout span overlaps the viewport, yielding
    /// each one's viewport-relative position and clipped height.
    ///
    /// This is the high-level entry point for rendering: it folds the
    /// scroll subtraction, top/bottom clipping, zero-height skipping,
    /// and row-index counting into one iterator so callers don't have
    /// to.
    ///
    /// Iteration stops early once an item begins at or below the
    /// viewport's bottom edge — long lists don't pay for offscreen
    /// items.
    pub fn visible_items(&self, scroll: u16, viewport_height: u16) -> VisibleIter<'_, T> {
        VisibleIter {
            walk: Box::new(self.walk()),
            scroll,
            viewport_height,
            row_idx_counter: 0,
            finished: false,
        }
    }

    /// Walk every item with its top y offset. The offsets honor collapsed
    /// sections (a collapsed section's rows sit at zero effective height), so
    /// for the effective height of an item prefer [`visible_items`]; an item's
    /// `height` field always reports its *declared* height.
    ///
    /// [`visible_items`]: SectionedList::visible_items
    pub fn iter_with_y(&self) -> impl Iterator<Item = (u16, &Item<T>)> {
        self.walk().map(|(y, it, _)| (y, it))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let kinds: Vec<(ItemKind, u16)> =
            list.items().iter().map(|it| (it.kind, it.height)).collect();
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
        assert_eq!(
            (v[1].viewport_y, v[1].visible_height, v[1].row_idx),
            (3, 5, Some(1))
        );
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
}
