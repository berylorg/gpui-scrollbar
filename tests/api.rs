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
    let owner = gpui_scrollbar::ScrollbarOwnerKey::new(
        gpui_scrollbar::ScrollbarOwnerId::new(1),
        gpui_scrollbar::ScrollbarMountGeneration::new(1),
    );

    assert_eq!(
        policy.opacity_for_owner_at(owner, true, std::time::Instant::now()),
        Some(1.0)
    );
    assert_eq!(
        policy.opacity_for_owner_at(owner, false, std::time::Instant::now()),
        None
    );
}

#[test]
fn public_keyed_identity_and_visibility_api_is_consumer_usable() {
    let owner = gpui_scrollbar::ScrollbarOwnerKey::new(
        gpui_scrollbar::ScrollbarOwnerId::new(9),
        gpui_scrollbar::ScrollbarMountGeneration::new(4),
    );
    let state = gpui_scrollbar::ScrollbarState::new(owner);
    let first = state
        .record_activity_at(owner, std::time::Instant::now())
        .expect("current owner activity");
    let second = state
        .record_activity_at(owner, std::time::Instant::now())
        .expect("next current owner activity");

    assert_eq!(first.owner, owner);
    assert!(second.sequence > first.sequence);
    assert_eq!(state.current_owner(), Some(owner));
}
