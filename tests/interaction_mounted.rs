mod support;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui::{Modifiers, MouseButton, ScrollHandle, point, px};
use gpui_scrollbar::{Axis, ScrollbarState, ScrollbarStyle, scrollbar_geometry_snapshot};
use support::{
    MountedScrollHandleView, MountedScrollbarView, has_active_drag, interaction, key,
    vertical_state,
};

#[gpui::test]
fn owner_replacement_unmount_and_window_teardown_cancel_exactly_once(
    cx: &mut gpui::TestAppContext,
) {
    let cx = cx.add_empty_window();
    let owner = key(1, 1);
    let replacement = key(2, 1);
    let style = ScrollbarStyle::default();
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
    let first = state
        .start_drag(&interaction, style, snapshot, px(4.0))
        .expect("first drag starts");

    cx.update(|window, cx| {
        assert!(state.replace_owner(owner, replacement, window, cx));
        assert!(!state.replace_owner(owner, key(3, 1), window, cx));
    });
    assert!(!state.complete_drag(&first));
    assert_eq!(state.current_owner(), Some(replacement));

    current.set(vertical_state(replacement, px(0.0)));
    let snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();
    let second = state
        .start_drag(&interaction, style, snapshot, px(4.0))
        .expect("second drag starts");
    cx.update(|window, cx| assert!(state.unmount_scrollbar(replacement, window, cx)));
    assert!(!state.complete_drag(&second));
    assert_eq!(state.current_owner(), None);

    assert!(state.mount(key(4, 1)));
    cx.update(|window, cx| assert!(state.teardown_window(key(4, 1), window, cx)));
    assert_eq!(state.current_owner(), None);
}

#[gpui::test]
fn mounted_drag_release_completes_once_and_repeated_release_is_obsolete(
    cx: &mut gpui::TestAppContext,
) {
    let owner = key(11, 4);
    let current = Rc::new(Cell::new(vertical_state(owner, px(120.0))));
    let offsets = Rc::new(RefCell::new(Vec::new()));
    let started = Rc::new(Cell::new(0));
    let completed = Rc::new(Cell::new(0));
    let interaction = interaction(
        current,
        offsets.clone(),
        Default::default(),
        started.clone(),
        completed.clone(),
    );
    let state = ScrollbarState::new(owner);
    let (view, cx) = cx.add_window_view({
        let state = state.clone();
        move |_, _| MountedScrollbarView {
            state,
            interaction,
            axis: Axis::Vertical,
        }
    });
    let press = point(px(232.0), px(100.0));
    let first_move = point(px(232.0), px(125.0));
    let second_move = point(px(232.0), px(150.0));
    let third_move = point(px(232.0), px(175.0));

    cx.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(first_move, MouseButton::Left, Modifiers::none());
    assert_eq!(started.get(), 1);
    assert!(has_active_drag(cx));
    view.update(cx, |_, cx| cx.notify());
    cx.update(|window, cx| window.draw_and_present_for_test(cx));
    assert!(has_active_drag(cx));

    cx.simulate_mouse_move(second_move, MouseButton::Left, Modifiers::none());
    assert!(has_active_drag(cx));
    assert_eq!(offsets.borrow().len(), 1);
    view.update(cx, |_, cx| cx.notify());
    cx.update(|window, cx| window.draw_and_present_for_test(cx));
    assert!(has_active_drag(cx));

    cx.simulate_mouse_move(third_move, MouseButton::Left, Modifiers::none());
    assert_eq!(offsets.borrow().len(), 2);
    view.update(cx, |_, cx| cx.notify());
    cx.update(|window, cx| window.draw_and_present_for_test(cx));
    assert!(has_active_drag(cx));

    cx.simulate_mouse_up(third_move, MouseButton::Left, Modifiers::none());
    assert_eq!(completed.get(), 1);
    assert!(!has_active_drag(cx));

    cx.simulate_mouse_up(third_move, MouseButton::Left, Modifiers::none());
    assert_eq!(completed.get(), 1);
}

#[gpui::test]
fn mounted_same_scroll_handle_redraw_preserves_drag_until_release(cx: &mut gpui::TestAppContext) {
    let owner = key(12, 1);
    let first_handle = ScrollHandle::new();
    let second_handle = ScrollHandle::new();
    let state = ScrollbarState::new(owner);
    let (view, cx) = cx.add_window_view({
        let state = state.clone();
        let first_handle = first_handle.clone();
        let second_handle = second_handle.clone();
        move |_, _| MountedScrollHandleView {
            state,
            first_handle,
            second_handle,
            use_second_handle: false,
        }
    });
    first_handle.set_offset(point(px(0.0), px(-120.0)));
    second_handle.set_offset(point(px(0.0), px(-120.0)));
    view.update(cx, |_, cx| cx.notify());
    cx.update(|window, cx| window.draw_and_present_for_test(cx));

    let press = point(px(232.0), px(100.0));
    let first_move = point(px(232.0), px(125.0));
    let second_move = point(px(232.0), px(150.0));
    let third_move = point(px(232.0), px(175.0));
    cx.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(first_move, MouseButton::Left, Modifiers::none());
    assert!(has_active_drag(cx));
    let first_offset = first_handle.offset().y;

    view.update(cx, |_, cx| cx.notify());
    cx.update(|window, cx| window.draw_and_present_for_test(cx));
    assert!(has_active_drag(cx));
    cx.simulate_mouse_move(second_move, MouseButton::Left, Modifiers::none());
    assert!(first_handle.offset().y < first_offset);
    let second_offset = first_handle.offset().y;
    view.update(cx, |_, cx| cx.notify());
    cx.update(|window, cx| window.draw_and_present_for_test(cx));
    assert!(has_active_drag(cx));

    cx.simulate_mouse_move(third_move, MouseButton::Left, Modifiers::none());
    assert!(first_handle.offset().y < second_offset);
    view.update(cx, |_, cx| cx.notify());
    cx.update(|window, cx| window.draw_and_present_for_test(cx));
    assert!(has_active_drag(cx));

    cx.simulate_mouse_up(third_move, MouseButton::Left, Modifiers::none());
    assert!(!has_active_drag(cx));
}

#[gpui::test]
fn mounted_distinct_equal_state_scroll_handle_replacement_rejects_pending_press(
    cx: &mut gpui::TestAppContext,
) {
    let owner = key(13, 1);
    let first_handle = ScrollHandle::new();
    let second_handle = ScrollHandle::new();
    let state = ScrollbarState::new(owner);
    let (view, cx) = cx.add_window_view({
        let state = state.clone();
        let first_handle = first_handle.clone();
        let second_handle = second_handle.clone();
        move |_, _| MountedScrollHandleView {
            state,
            first_handle,
            second_handle,
            use_second_handle: false,
        }
    });
    first_handle.set_offset(point(px(0.0), px(-120.0)));
    second_handle.set_offset(point(px(0.0), px(-120.0)));
    view.update(cx, |_, cx| cx.notify());
    cx.update(|window, cx| window.draw_and_present_for_test(cx));
    assert_eq!(first_handle.bounds().size, second_handle.bounds().size);
    assert_eq!(first_handle.max_offset(), second_handle.max_offset());
    assert_eq!(first_handle.offset(), second_handle.offset());

    cx.simulate_mouse_down(
        point(px(232.0), px(100.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    view.update(cx, |view, cx| {
        view.use_second_handle = true;
        cx.notify();
    });
    cx.update(|window, cx| window.draw_and_present_for_test(cx));
    cx.simulate_mouse_move(
        point(px(232.0), px(125.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.run_until_parked();

    assert!(!has_active_drag(cx));
    assert_eq!(first_handle.offset().y, px(-120.0));
    assert_eq!(second_handle.offset().y, px(-120.0));
}

#[gpui::test]
fn mounted_owner_replacement_capture_cancel_releases_without_owner_callback(
    cx: &mut gpui::TestAppContext,
) {
    let owner = key(21, 2);
    let replacement = key(22, 1);
    let current = Rc::new(Cell::new(vertical_state(owner, px(120.0))));
    let started = Rc::new(Cell::new(0));
    let completed = Rc::new(Cell::new(0));
    let interaction = interaction(
        current,
        Default::default(),
        Default::default(),
        started.clone(),
        completed.clone(),
    );
    let state = ScrollbarState::new(owner);
    let (_, cx) = cx.add_window_view({
        let state = state.clone();
        move |_, _| MountedScrollbarView {
            state,
            interaction,
            axis: Axis::Vertical,
        }
    });

    cx.simulate_mouse_down(
        point(px(232.0), px(100.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.simulate_mouse_move(
        point(px(232.0), px(125.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    assert_eq!(started.get(), 1);
    cx.update(|window, cx| assert!(state.replace_owner(owner, replacement, window, cx)));

    assert!(!has_active_drag(cx));
    assert_eq!(completed.get(), 0);
    cx.simulate_mouse_up(
        point(px(232.0), px(125.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    assert_eq!(completed.get(), 0);
}

#[gpui::test]
fn mounted_vertical_lane_hold_never_installs_active_drag(cx: &mut gpui::TestAppContext) {
    let owner = key(40, 1);
    let current = Rc::new(Cell::new(vertical_state(owner, px(120.0))));
    let pages = Rc::new(RefCell::new(Vec::new()));
    let interaction = interaction(
        current,
        Default::default(),
        pages.clone(),
        Default::default(),
        Default::default(),
    );
    let state = ScrollbarState::new(owner);
    let (_, cx) = cx.add_window_view({
        let state = state.clone();
        move |_, _| MountedScrollbarView {
            state,
            interaction,
            axis: Axis::Vertical,
        }
    });
    let press = point(px(236.0), px(220.0));

    cx.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(
        point(px(236.0), px(200.0)),
        MouseButton::Left,
        Modifiers::none(),
    );

    assert!(!has_active_drag(cx));
    assert_eq!(pages.borrow().len(), 1);
}

#[gpui::test]
fn mounted_stale_geometry_at_threshold_never_retains_active_drag(cx: &mut gpui::TestAppContext) {
    let owner = key(50, 1);
    let current = Rc::new(Cell::new(vertical_state(owner, px(120.0))));
    let started = Rc::new(Cell::new(0));
    let interaction = interaction(
        current.clone(),
        Default::default(),
        Default::default(),
        started.clone(),
        Default::default(),
    );
    let state = ScrollbarState::new(owner);
    let (_, cx) = cx.add_window_view({
        let state = state.clone();
        move |_, _| MountedScrollbarView {
            state,
            interaction,
            axis: Axis::Vertical,
        }
    });
    let press = point(px(232.0), px(100.0));
    cx.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    current.set(vertical_state(owner, px(121.0)));
    cx.simulate_mouse_move(
        point(px(232.0), px(125.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.run_until_parked();

    assert!(!has_active_drag(cx));
    assert_eq!(started.get(), 0);
}

#[gpui::test]
fn mounted_pending_threshold_owner_replacement_cannot_start_replacement_drag(
    cx: &mut gpui::TestAppContext,
) {
    let owner = key(51, 1);
    let replacement = key(51, 2);
    let current = Rc::new(Cell::new(vertical_state(owner, px(120.0))));
    let started = Rc::new(Cell::new(0));
    let interaction = interaction(
        current,
        Default::default(),
        Default::default(),
        started.clone(),
        Default::default(),
    );
    let state = ScrollbarState::new(owner);
    let (_, cx) = cx.add_window_view({
        let state = state.clone();
        move |_, _| MountedScrollbarView {
            state,
            interaction,
            axis: Axis::Vertical,
        }
    });
    cx.simulate_mouse_down(
        point(px(232.0), px(100.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.update(|window, cx| assert!(state.replace_owner(owner, replacement, window, cx)));
    cx.simulate_mouse_move(
        point(px(232.0), px(125.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.run_until_parked();

    assert!(!has_active_drag(cx));
    assert_eq!(started.get(), 0);
    assert_eq!(state.current_owner(), Some(replacement));
}

#[gpui::test]
fn mounted_pending_press_cannot_promote_through_rerender_and_geometry_reversion(
    cx: &mut gpui::TestAppContext,
) {
    let owner = key(52, 1);
    let original = vertical_state(owner, px(120.0));
    let current = Rc::new(Cell::new(original));
    let started = Rc::new(Cell::new(0));
    let interaction = interaction(
        current.clone(),
        Default::default(),
        Default::default(),
        started.clone(),
        Default::default(),
    );
    let state = ScrollbarState::new(owner);
    let (view, cx) = cx.add_window_view({
        let state = state.clone();
        move |_, _| MountedScrollbarView {
            state,
            interaction,
            axis: Axis::Vertical,
        }
    });
    let press = point(px(232.0), px(100.0));

    cx.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    current.set(vertical_state(owner, px(121.0)));
    view.update(cx, |_, cx| cx.notify());
    cx.update(|window, cx| window.draw_and_present_for_test(cx));
    current.set(original);
    view.update(cx, |_, cx| cx.notify());
    cx.update(|window, cx| window.draw_and_present_for_test(cx));

    cx.simulate_mouse_move(
        point(px(232.0), px(125.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.run_until_parked();

    assert!(!has_active_drag(cx));
    assert_eq!(started.get(), 0);
}

#[gpui::test]
fn mounted_active_drag_stale_move_directly_stops_gpui_capture(cx: &mut gpui::TestAppContext) {
    let owner = key(53, 1);
    let stale_owner = key(53, 2);
    let current = Rc::new(Cell::new(vertical_state(owner, px(120.0))));
    let offsets = Rc::new(RefCell::new(Vec::new()));
    let interaction = interaction(
        current.clone(),
        offsets.clone(),
        Default::default(),
        Default::default(),
        Default::default(),
    );
    let state = ScrollbarState::new(owner);
    let (_, cx) = cx.add_window_view({
        let state = state.clone();
        move |_, _| MountedScrollbarView {
            state,
            interaction,
            axis: Axis::Vertical,
        }
    });
    let press = point(px(232.0), px(100.0));

    cx.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(
        point(px(232.0), px(125.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    assert!(has_active_drag(cx));
    let update_count = offsets.borrow().len();

    current.set(vertical_state(stale_owner, px(120.0)));
    cx.simulate_mouse_move(
        point(px(232.0), px(150.0)),
        MouseButton::Left,
        Modifiers::none(),
    );

    assert!(!has_active_drag(cx));
    assert_eq!(offsets.borrow().len(), update_count);
}
