use gpui_scrollbar::{Axis, LaneClick, ScrollDirection};

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
