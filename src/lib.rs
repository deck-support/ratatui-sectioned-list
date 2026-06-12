//! Layout, focus, scroll, and hit-test primitives for a sectioned list —
//! a list of non-focusable headers interleaved with focusable variable-height
//! rows.
//!
//! Built for ratatui: the default `ratatui` feature ships a [`StatefulWidget`]
//! wrapper plus a batteries-included [`BasicItem`] preset, so a sectioned list
//! works out of the box — see the [`widget`] module.
//!
//! The geometry underneath is UI-framework agnostic: heights and offsets are
//! plain `u16` values measured in terminal rows, and the caller renders. If you
//! only want that core, opt out with `default-features = false` for a
//! zero-dependency build with no ratatui types in sight.
//!
//! [`StatefulWidget`]: https://docs.rs/ratatui/latest/ratatui/widgets/trait.StatefulWidget.html
//! [`BasicItem`]: widget::BasicItem

#[cfg(feature = "ratatui")]
pub mod widget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Header,
    Row,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DividerLayout {
    /// Width available for the rendered label after reserving the leading
    /// cells, chevron, rule spacer, optional badge, and right-side actions.
    pub label_width: usize,
    /// Width of the fill rule between the label and the optional badge/actions.
    pub rule_width: usize,
    /// Cell range of the optional badge text, excluding its leading gap.
    pub badge: Option<std::ops::Range<usize>>,
    /// Cell ranges of right-side actions in the same order they were requested.
    pub actions: Vec<std::ops::Range<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DividerLayoutSpec {
    pub width: usize,
    pub leading_width: usize,
    /// Width of the collapse indicator plus its trailing space.
    pub chevron_width: usize,
    /// Width of the spacer between the label and rule.
    pub spacer_width: usize,
    /// Width of the gap before each optional badge/action.
    pub gap_width: usize,
    /// Display width of the untruncated label.
    pub label_width: usize,
    /// Display width of the optional badge text, excluding its leading gap.
    pub badge_width: Option<usize>,
    /// Widths of right-aligned action labels/buttons in display order.
    pub action_widths: Vec<usize>,
}

/// Lay out a one-line section divider with a leading region, chevron, label,
/// rule fill, optional badge, and right-aligned actions.
///
/// This is UI-framework agnostic: callers provide display widths in terminal
/// cells and render text/styles themselves from the returned widths/ranges.
pub fn layout_divider(spec: DividerLayoutSpec) -> DividerLayout {
    let actions_width: usize = spec
        .action_widths
        .iter()
        .map(|w| spec.gap_width.saturating_add(*w))
        .sum();
    let avail = spec
        .width
        .saturating_sub(spec.leading_width)
        .saturating_sub(spec.chevron_width)
        .saturating_sub(actions_width);

    let badge_total = spec
        .badge_width
        .filter(|w| {
            avail
                > spec
                    .gap_width
                    .saturating_add(*w)
                    .saturating_add(spec.spacer_width)
        })
        .map(|w| spec.gap_width.saturating_add(w))
        .unwrap_or(0);
    let label_width = spec.label_width.min(
        avail
            .saturating_sub(spec.spacer_width)
            .saturating_sub(badge_total),
    );
    let rule_width = avail
        .saturating_sub(label_width)
        .saturating_sub(spec.spacer_width)
        .saturating_sub(badge_total);

    let mut cursor =
        spec.leading_width + spec.chevron_width + label_width + spec.spacer_width + rule_width;
    let badge = spec.badge_width.and_then(|w| {
        if badge_total == 0 {
            return None;
        }
        cursor += spec.gap_width;
        let range = cursor..cursor.saturating_add(w);
        cursor = range.end;
        Some(range)
    });

    let mut actions = Vec::with_capacity(spec.action_widths.len());
    for width in spec.action_widths {
        cursor += spec.gap_width;
        let range = cursor..cursor.saturating_add(width);
        cursor = range.end;
        actions.push(range);
    }

    DividerLayout {
        label_width,
        rule_width,
        badge,
        actions,
    }
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
    /// Leading rows of `height` that are blank/inert: they occupy layout space
    /// but are excluded from hit-testing, so a click there returns nothing.
    /// Used for a header's top margin (section spacing). 0 for a normal item.
    /// Render these as blank rows above the item's content.
    pub lead: u16,
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
    /// Offset inside the item's declared height where this visible slice
    /// begins. `0` means the item is visible from its first line; a non-zero
    /// value means the viewport clipped the item's top.
    pub item_y_offset: u16,
    /// `Some(row_idx)` for focusable rows; `None` for headers. The row
    /// index is the global one — counted across the whole list, with
    /// headers skipped, preserved across rows that are above the
    /// viewport or have height 0.
    pub row_idx: Option<usize>,
}

impl<T> Visible<'_, T> {
    /// Map a line inside this item to a viewport-relative y coordinate.
    ///
    /// Returns `None` when `item_line` is clipped above or below the viewport.
    /// This lets renderers build per-item click regions without first
    /// rendering a full offscreen list and translating global line indices.
    pub fn viewport_y_for_item_line(&self, item_line: u16) -> Option<u16> {
        let visible_end = self.item_y_offset.saturating_add(self.visible_height);
        if item_line < self.item_y_offset || item_line >= visible_end {
            return None;
        }
        Some(self.viewport_y + item_line - self.item_y_offset)
    }
}

/// Iterator returned by [`SectionedList::visible_items`].
///
/// Built on top of the internal `walk` traversal, so the effective-height /
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
                item_y_offset: overlap_top - item_top,
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

/// A row payload that knows its own rendered height in terminal rows, so it can
/// be added with [`SectionedList::push_row_auto`] instead of a hand-counted
/// height. With the `ratatui` feature, [`BasicItem`] implements this as
/// `1 + lines.len()`.
///
/// [`BasicItem`]: widget::BasicItem
pub trait RowHeight {
    /// Height in terminal rows.
    fn row_height(&self) -> u16;
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
            lead: 0,
        });
    }

    pub fn push_row(&mut self, data: T, height: u16) {
        self.items.push(Item {
            kind: ItemKind::Row,
            height,
            data,
            collapsed: false,
            lead: 0,
        });
    }

    /// Add a header at the default height of one terminal row. Use
    /// [`push_header`](Self::push_header) when you want a taller header, or
    /// [`push_header_margin`](Self::push_header_margin) for top margin.
    pub fn push_header_auto(&mut self, data: T) {
        self.push_header(data, 1);
    }

    /// Add a one-row header bar preceded by `margin_top` blank rows of section
    /// spacing. The blank rows count toward layout height but are excluded from
    /// hit-testing — a click on them returns nothing, only the bar toggles the
    /// section. Total height is `margin_top + 1`.
    pub fn push_header_margin(&mut self, data: T, margin_top: u16) {
        self.items.push(Item {
            kind: ItemKind::Header,
            height: margin_top + 1,
            data,
            collapsed: false,
            lead: margin_top,
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
                // Skip the leading inert (margin) rows: only the bar below them
                // toggles the section.
                let bar_top = top.saturating_add(it.lead.min(eff));
                let bottom = top.saturating_add(eff);
                if layout_y >= bar_top && layout_y < bottom {
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

impl<T: RowHeight> SectionedList<T> {
    /// Add a row sized to [`RowHeight::row_height`] — no hand-counted height.
    pub fn push_row_auto(&mut self, data: T) {
        let height = data.row_height();
        self.push_row(data, height);
    }
}

#[cfg(test)]
mod tests;
