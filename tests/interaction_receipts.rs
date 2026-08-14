mod support;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui::{point, px, size};
use gpui_scrollbar::{
    Axis, LaneClick, ScrollDirection, ScrollbarInteraction, ScrollbarPointerDownAction,
    ScrollbarState, ScrollbarStyle, dispatch_scrollbar_pointer_down, scrollbar_geometry_snapshot,
    scrollbar_pointer_down_action,
};
use support::{interaction, key, vertical_state};

#[test]
fn exact_snapshot_contains_all_same_record_facts() {
    let owner = key(7, 3);
    let snapshot = scrollbar_geometry_snapshot(
        Axis::Vertical,
        ScrollbarStyle::default().geometry,
        vertical_state(owner, px(500.0)),
    )
    .expect("overflow snapshot");

    assert_eq!(snapshot.owner, owner);
    assert_eq!(snapshot.axis, Axis::Vertical);
    assert_eq!(snapshot.viewport_length, px(240.0));
    assert_eq!(snapshot.content_length, px(480.0));
    assert_eq!(snapshot.scroll_offset, px(240.0));
    assert_eq!(snapshot.track_bounds.start, px(6.0));
    assert_eq!(snapshot.track_bounds.end, px(234.0));
    assert_eq!(snapshot.page_distance, px(180.0));
}

#[test]
fn vertical_lane_uses_hit_tested_snapshot_direction_and_distance() {
    let owner = key(1, 1);
    let style = ScrollbarStyle::default();
    let snapshot = scrollbar_geometry_snapshot(
        Axis::Vertical,
        style.geometry,
        vertical_state(owner, px(120.0)),
    )
    .unwrap();
    let before = scrollbar_pointer_down_action(
        snapshot,
        point(px(236.0), snapshot.thumb_bounds.start - px(1.0)),
    );
    let after = scrollbar_pointer_down_action(
        snapshot,
        point(px(236.0), snapshot.thumb_bounds.end + px(1.0)),
    );

    assert!(matches!(before, ScrollbarPointerDownAction::Page {
        snapshot: hit, lane: LaneClick::BeforeThumb,
        direction: ScrollDirection::Backward, distance
    } if hit == snapshot && distance == px(180.0)));
    assert!(matches!(after, ScrollbarPointerDownAction::Page {
        snapshot: hit, lane: LaneClick::AfterThumb,
        direction: ScrollDirection::Forward, distance
    } if hit == snapshot && distance == px(180.0)));
}

#[test]
fn stale_geometry_and_owner_reject_lane_without_callback() {
    let owner = key(1, 1);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(owner, px(120.0))));
    let old = scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();
    let pages = Rc::new(RefCell::new(Vec::new()));
    let interaction = interaction(
        current.clone(),
        Default::default(),
        pages.clone(),
        Default::default(),
        Default::default(),
    );
    let retained = ScrollbarState::new(owner);

    current.set(vertical_state(owner, px(121.0)));
    let result = dispatch_scrollbar_pointer_down(
        &retained,
        style,
        &interaction,
        old,
        point(px(236.0), old.thumb_bounds.end + px(1.0)),
    );
    assert_eq!(result, ScrollbarPointerDownAction::Ignore);

    current.set(vertical_state(key(2, 1), px(120.0)));
    let result = dispatch_scrollbar_pointer_down(
        &retained,
        style,
        &interaction,
        old,
        point(px(236.0), old.thumb_bounds.end + px(1.0)),
    );
    assert_eq!(result, ScrollbarPointerDownAction::Ignore);
    assert!(pages.borrow().is_empty());
}

#[test]
fn stale_mount_generation_rejects_lane_without_callback() {
    let mounted = key(1, 2);
    let stale = key(1, 1);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(stale, px(120.0))));
    let snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();
    let pages = Rc::new(RefCell::new(Vec::new()));
    let interaction = interaction(
        current,
        Default::default(),
        pages.clone(),
        Default::default(),
        Default::default(),
    );
    let retained = ScrollbarState::new(mounted);

    assert_eq!(
        dispatch_scrollbar_pointer_down(
            &retained,
            style,
            &interaction,
            snapshot,
            point(px(236.0), snapshot.thumb_bounds.end + px(1.0)),
        ),
        ScrollbarPointerDownAction::Ignore
    );
    assert!(pages.borrow().is_empty());
}

#[test]
fn lane_page_returns_fresh_post_mutation_snapshot() {
    let owner = key(8, 1);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(owner, px(120.0))));
    let interaction = ScrollbarInteraction::new(
        {
            let current = current.clone();
            move || Some(current.get())
        },
        |_, _| {},
        {
            let current = current.clone();
            move |_, _, distance| {
                let mut next = current.get();
                next.scroll_offset.y += distance;
                current.set(next);
            }
        },
        |_| {},
        |_| {},
        |_, _, _| {},
    );
    let state = ScrollbarState::new(owner);
    let before =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();

    let result = dispatch_scrollbar_pointer_down(
        &state,
        style,
        &interaction,
        before,
        point(px(236.0), before.thumb_bounds.end + px(1.0)),
    );
    let after = scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();

    assert!(matches!(result, ScrollbarPointerDownAction::Page {
        snapshot, direction: ScrollDirection::Forward, distance, ..
    } if snapshot == after && snapshot != before && distance == before.page_distance));
}

#[test]
fn lane_page_owner_change_suppresses_post_mutation_action() {
    let owner = key(9, 1);
    let replacement = key(9, 2);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(owner, px(120.0))));
    let pages = Rc::new(Cell::new(0));
    let interaction = ScrollbarInteraction::new(
        {
            let current = current.clone();
            move || Some(current.get())
        },
        |_, _| {},
        {
            let current = current.clone();
            let pages = pages.clone();
            move |_, _, _| {
                pages.set(pages.get() + 1);
                current.set(vertical_state(replacement, px(0.0)));
            }
        },
        |_| {},
        |_| {},
        |_, _, _| {},
    );
    let state = ScrollbarState::new(owner);
    let snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();

    assert_eq!(
        dispatch_scrollbar_pointer_down(
            &state,
            style,
            &interaction,
            snapshot,
            point(px(236.0), snapshot.thumb_bounds.end + px(1.0)),
        ),
        ScrollbarPointerDownAction::Ignore
    );
    assert_eq!(pages.get(), 1);
}

#[test]
fn lane_page_loss_of_overflow_suppresses_post_mutation_action() {
    let owner = key(10, 1);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(owner, px(120.0))));
    let interaction = ScrollbarInteraction::new(
        {
            let current = current.clone();
            move || Some(current.get())
        },
        |_, _| {},
        {
            let current = current.clone();
            move |_, _, _| {
                let mut next = current.get();
                next.content_size = size(px(240.0), px(240.0));
                current.set(next);
            }
        },
        |_| {},
        |_| {},
        |_, _, _| {},
    );
    let state = ScrollbarState::new(owner);
    let snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();

    assert_eq!(
        dispatch_scrollbar_pointer_down(
            &state,
            style,
            &interaction,
            snapshot,
            point(px(236.0), snapshot.thumb_bounds.end + px(1.0)),
        ),
        ScrollbarPointerDownAction::Ignore
    );
}

#[test]
fn horizontal_outside_thumb_dispatches_nothing() {
    let owner = key(1, 1);
    let style = ScrollbarStyle::default();
    let mut current_state = vertical_state(owner, px(120.0));
    current_state.content_size = size(px(480.0), px(240.0));
    current_state.scroll_offset = point(px(120.0), px(0.0));
    let current = Rc::new(Cell::new(current_state));
    let snapshot =
        scrollbar_geometry_snapshot(Axis::Horizontal, style.geometry, current.get()).unwrap();
    let pages = Rc::new(RefCell::new(Vec::new()));
    let interaction = interaction(
        current,
        Default::default(),
        pages.clone(),
        Default::default(),
        Default::default(),
    );
    let retained = ScrollbarState::new(owner);

    for x in [
        snapshot.thumb_bounds.start - px(1.0),
        snapshot.thumb_bounds.end + px(1.0),
    ] {
        assert_eq!(
            dispatch_scrollbar_pointer_down(
                &retained,
                style,
                &interaction,
                snapshot,
                point(x, px(236.0)),
            ),
            ScrollbarPointerDownAction::Ignore
        );
    }
    assert!(pages.borrow().is_empty());
}

#[test]
fn custom_renderer_receipts_reject_late_instance_a_after_same_owner_instance_b() {
    let owner = key(1, 1);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(owner, px(0.0))));
    let snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();
    let offsets = Rc::new(RefCell::new(Vec::new()));
    let started = Rc::new(Cell::new(0));
    let completed = Rc::new(Cell::new(0));
    let interaction = interaction(
        current.clone(),
        offsets.clone(),
        Default::default(),
        started.clone(),
        completed.clone(),
    );
    let retained = ScrollbarState::new(owner);

    let first = retained
        .start_drag(&interaction, style, snapshot, px(10.0))
        .expect("instance A starts");
    let pointer = point(px(236.0), snapshot.track_bounds.start + px(57.0) + px(10.0));
    let (_, next) = retained
        .update_drag(&first, pointer)
        .expect("instance A updates");
    assert_eq!(next, px(120.0));
    assert_eq!(offsets.borrow().as_slice(), &[px(120.0)]);
    assert!(retained.complete_drag(&first));

    current.set(vertical_state(owner, px(0.0)));
    let second_snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();
    let second = retained
        .start_drag(&interaction, style, second_snapshot, px(10.0))
        .expect("instance B starts");
    assert!(retained.update_drag(&first, pointer).is_none());
    assert!(!retained.complete_drag(&first));
    assert!(!retained.cancel_drag(&first));
    assert_eq!(offsets.borrow().as_slice(), &[px(120.0)]);

    let (_, next) = retained
        .update_drag(&second, pointer)
        .expect("instance B remains updateable");
    assert_eq!(next, px(120.0));
    assert!(retained.complete_drag(&second));
    assert!(!retained.complete_drag(&second));
    assert_eq!(started.get(), 2);
    assert_eq!(completed.get(), 2);
}

#[test]
fn receipt_from_distinct_state_cannot_address_same_numbered_drag() {
    let style = ScrollbarStyle::default();
    let first_owner = key(3, 1);
    let second_owner = key(4, 1);
    let first_current = Rc::new(Cell::new(vertical_state(first_owner, px(0.0))));
    let second_current = Rc::new(Cell::new(vertical_state(second_owner, px(0.0))));
    let first = ScrollbarState::new(first_owner);
    let second = ScrollbarState::new(second_owner);
    let first_interaction = interaction(
        first_current.clone(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
    );
    let second_offsets = Rc::new(RefCell::new(Vec::new()));
    let second_completed = Rc::new(Cell::new(0));
    let second_interaction = interaction(
        second_current.clone(),
        second_offsets.clone(),
        Default::default(),
        Default::default(),
        second_completed.clone(),
    );
    let first_snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, first_current.get()).unwrap();
    let second_snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, second_current.get()).unwrap();
    let first_receipt = first
        .start_drag(&first_interaction, style, first_snapshot, px(10.0))
        .expect("state A instance one starts");
    let second_receipt = second
        .start_drag(&second_interaction, style, second_snapshot, px(10.0))
        .expect("state B instance one starts");
    let pointer = point(px(236.0), second_snapshot.track_bounds.start + px(67.0));

    assert!(second.update_drag(&first_receipt, pointer).is_none());
    assert!(!second.complete_drag(&first_receipt));
    assert!(!second.cancel_drag(&first_receipt));
    assert!(second_offsets.borrow().is_empty());
    assert_eq!(second_completed.get(), 0);
    assert!(second.update_drag(&second_receipt, pointer).is_some());
    assert!(second.complete_drag(&second_receipt));
    assert_eq!(second_completed.get(), 1);
}

#[test]
fn receipt_does_not_keep_its_issuing_state_usable_after_drop() {
    let style = ScrollbarStyle::default();
    let owner = key(5, 1);
    let current = Rc::new(Cell::new(vertical_state(owner, px(0.0))));
    let interaction = interaction(
        current.clone(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
    );
    let state = ScrollbarState::new(owner);
    let snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();
    let receipt = state
        .start_drag(&interaction, style, snapshot, px(10.0))
        .expect("receipt issued");
    assert!(!receipt.is_orphaned());
    drop(state);
    assert!(receipt.is_orphaned());

    let replacement = ScrollbarState::new(owner);
    assert!(
        replacement
            .update_drag(&receipt, point(px(236.0), snapshot.thumb_bounds.start))
            .is_none()
    );
    assert!(!replacement.complete_drag(&receipt));
    assert!(!replacement.cancel_drag(&receipt));
}

#[test]
fn reentrant_a_update_cannot_write_back_into_recurrent_instance_b() {
    let owner = key(2, 1);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(owner, px(0.0))));
    let retained = ScrollbarState::new(owner);
    let first_receipt = Rc::new(RefCell::new(None));
    let second_receipt = Rc::new(RefCell::new(None));
    let interaction_slot = Rc::new(RefCell::new(None));
    let callback_count = Rc::new(Cell::new(0));
    let reentered = Rc::new(Cell::new(false));
    let publish_stale_once = Rc::new(Cell::new(false));
    let interaction = ScrollbarInteraction::new(
        {
            let current = current.clone();
            let publish_stale_once = publish_stale_once.clone();
            move || {
                let mut state = current.get();
                if publish_stale_once.replace(false) {
                    state.scroll_offset.y = px(11.0);
                }
                Some(state)
            }
        },
        {
            let current = current.clone();
            let retained = retained.clone();
            let first_receipt = first_receipt.clone();
            let second_receipt = second_receipt.clone();
            let interaction_slot = interaction_slot.clone();
            let callback_count = callback_count.clone();
            let reentered = reentered.clone();
            let publish_stale_once = publish_stale_once.clone();
            move |_, _| {
                callback_count.set(callback_count.get() + 1);
                if reentered.replace(true) {
                    return;
                }
                let first = first_receipt.borrow();
                assert!(retained.complete_drag(first.as_deref().expect("instance A receipt")));
                let interaction = interaction_slot.borrow();
                let snapshot =
                    scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get())
                        .expect("instance B geometry");
                let second = retained
                    .start_drag(
                        interaction.as_ref().expect("interaction installed"),
                        style,
                        snapshot,
                        px(10.0),
                    )
                    .expect("instance B starts");
                second_receipt.replace(Some(Rc::new(second)));
                publish_stale_once.set(true);
            }
        },
        |_, _, _| {},
        |_| {},
        |_| {},
        |_, _, _| {},
    );
    interaction_slot.replace(Some(interaction.clone()));
    let snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();
    let first = Rc::new(
        retained
            .start_drag(&interaction, style, snapshot, px(10.0))
            .expect("instance A starts"),
    );
    first_receipt.replace(Some(first.clone()));
    let pointer = point(px(236.0), snapshot.track_bounds.start + px(57.0) + px(10.0));

    assert!(retained.update_drag(&first, pointer).is_some());
    let second = second_receipt.borrow();
    let second = second.as_deref().expect("instance B receipt");
    assert!(retained.update_drag(second, pointer).is_some());
    assert!(retained.complete_drag(second));
    assert_eq!(callback_count.get(), 2);
}

#[test]
fn stale_drag_geometry_cancels_exactly_once_without_scroll() {
    let owner = key(1, 1);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(owner, px(0.0))));
    let snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();
    let offsets = Rc::new(RefCell::new(Vec::new()));
    let interaction = interaction(
        current.clone(),
        offsets.clone(),
        Default::default(),
        Default::default(),
        Default::default(),
    );
    let retained = ScrollbarState::new(owner);
    let receipt = retained
        .start_drag(&interaction, style, snapshot, px(10.0))
        .expect("drag starts");

    current.set(vertical_state(owner, px(1.0)));
    assert!(
        retained
            .update_drag(&receipt, point(px(236.0), px(100.0)))
            .is_none()
    );
    assert!(!retained.cancel_drag(&receipt));
    assert!(!retained.complete_drag(&receipt));
    assert!(offsets.borrow().is_empty());
}

#[test]
fn stale_release_cancels_without_completion_callback() {
    let owner = key(1, 1);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(owner, px(0.0))));
    let snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();
    let completed = Rc::new(Cell::new(0));
    let interaction = interaction(
        current.clone(),
        Default::default(),
        Default::default(),
        Default::default(),
        completed.clone(),
    );
    let retained = ScrollbarState::new(owner);
    let receipt = retained
        .start_drag(&interaction, style, snapshot, px(10.0))
        .expect("drag starts");

    current.set(vertical_state(owner, px(1.0)));
    assert!(!retained.complete_drag(&receipt));
    assert!(!retained.complete_drag(&receipt));
    assert_eq!(completed.get(), 0);
}
