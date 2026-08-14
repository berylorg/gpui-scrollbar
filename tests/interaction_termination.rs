mod support;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui::{Modifiers, MouseButton, point, px, size};
use gpui_scrollbar::{Axis, ScrollbarState};
use support::{
    MountedScrollbarView, assert_mounted_active_drag_termination, has_active_drag, interaction,
    key, vertical_state,
};

#[gpui::test]
fn mounted_viewport_unmount_stops_active_capture(cx: &mut gpui::TestAppContext) {
    let owner = key(60, 1);
    assert_mounted_active_drag_termination(cx, owner, move |state, window, app| {
        assert!(state.unmount_viewport(owner, window, app));
    });
}

#[gpui::test]
fn mounted_scrollbar_unmount_stops_active_capture(cx: &mut gpui::TestAppContext) {
    let owner = key(61, 1);
    assert_mounted_active_drag_termination(cx, owner, move |state, window, app| {
        assert!(state.unmount_scrollbar(owner, window, app));
    });
}

#[gpui::test]
fn mounted_window_teardown_stops_active_capture(cx: &mut gpui::TestAppContext) {
    let owner = key(62, 1);
    assert_mounted_active_drag_termination(cx, owner, move |state, window, app| {
        assert!(state.teardown_window(owner, window, app));
    });
}

#[gpui::test]
fn mounted_capture_cancel_releases_retained_drag_without_completion(cx: &mut gpui::TestAppContext) {
    let owner = key(63, 1);
    assert_mounted_active_drag_termination(cx, owner, |_, window, app| {
        assert!(app.stop_active_drag(window));
    });
}

#[gpui::test]
fn mounted_horizontal_lane_hold_is_noop_without_active_drag(cx: &mut gpui::TestAppContext) {
    let owner = key(41, 1);
    let mut horizontal = vertical_state(owner, px(0.0));
    horizontal.content_size = size(px(480.0), px(240.0));
    horizontal.scroll_offset = point(px(120.0), px(0.0));
    let current = Rc::new(Cell::new(horizontal));
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
            axis: Axis::Horizontal,
        }
    });
    let press = point(px(220.0), px(236.0));

    cx.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(
        point(px(200.0), px(236.0)),
        MouseButton::Left,
        Modifiers::none(),
    );

    assert!(!has_active_drag(cx));
    assert!(pages.borrow().is_empty());
}

#[gpui::test]
fn semantic_rerender_release_settles_old_pending_and_fresh_press_can_drag(
    cx: &mut gpui::TestAppContext,
) {
    let owner = key(64, 1);
    let current = Rc::new(Cell::new(vertical_state(owner, px(120.0))));
    let started = Rc::new(Cell::new(0));
    let completed = Rc::new(Cell::new(0));
    let interaction = interaction(
        current.clone(),
        Default::default(),
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

    cx.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    current.set(vertical_state(owner, px(121.0)));
    view.update(cx, |_, cx| cx.notify());
    cx.update(|window, cx| window.draw_and_present_for_test(cx));
    cx.simulate_mouse_up(press, MouseButton::Left, Modifiers::none());
    assert_eq!(started.get(), 0);
    assert_eq!(completed.get(), 0);
    assert!(!has_active_drag(cx));

    cx.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    let moved = point(px(232.0), px(130.0));
    cx.simulate_mouse_move(moved, MouseButton::Left, Modifiers::none());
    assert_eq!(started.get(), 1);
    assert!(has_active_drag(cx));
    cx.simulate_mouse_up(moved, MouseButton::Left, Modifiers::none());
    assert_eq!(completed.get(), 1);
    assert!(!has_active_drag(cx));
}
