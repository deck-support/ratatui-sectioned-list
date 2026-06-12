use super::*;

#[test]
fn button_ranges_are_right_aligned_with_gaps() {
    // Two 3-cell buttons ("[x]") in a 20-wide header, 1-cell gap between:
    // total = 3 + 1 + 3 = 7, so they start at 20 - 7 = 13.
    let buttons = vec!["[a]".to_string(), "[b]".to_string()];
    let ranges = header_button_ranges(20, &buttons);
    assert_eq!(ranges, vec![13..16, 17..20]);
}

#[test]
fn button_ranges_empty_when_they_dont_fit() {
    let buttons = vec!["[a]".to_string(), "[b]".to_string()];
    assert!(header_button_ranges(6, &buttons).is_empty());
    assert!(header_button_ranges(0, &[]).is_empty());
}

#[test]
fn fill_str_spans_exact_width() {
    assert_eq!(display_width(&fill_str("─", 5)), 5);
    assert_eq!(fill_str("", 3), "   ");
    assert_eq!(fill_str("─", 0), "");
}
