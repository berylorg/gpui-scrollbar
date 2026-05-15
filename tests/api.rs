use gpui_scrollbar::{Axis, LaneClick, ScrollDirection, ScrollbarVisibilityPolicy};

#[test]
fn axis_helpers_identify_orientation() {
    assert!(Axis::Vertical.is_vertical());
    assert!(!Axis::Vertical.is_horizontal());
    assert!(Axis::Horizontal.is_horizontal());
    assert!(!Axis::Horizontal.is_vertical());
}

#[test]
fn lane_clicks_map_to_page_directions() {
    assert_eq!(
        LaneClick::BeforeThumb.page_direction(),
        ScrollDirection::Backward
    );
    assert_eq!(
        LaneClick::AfterThumb.page_direction(),
        ScrollDirection::Forward
    );
}

#[test]
fn always_visible_policy_requires_overflow() {
    let policy = ScrollbarVisibilityPolicy::always_visible();

    assert_eq!(policy.opacity_for_overflow(true), Some(1.0));
    assert_eq!(policy.opacity_for_overflow(false), None);
}
