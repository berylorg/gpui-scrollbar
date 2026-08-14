#![allow(dead_code)] // Integration targets share only the helpers each one needs.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui::prelude::*;
use gpui::{
    Bounds, Modifiers, MouseButton, Pixels, Render, ScrollHandle, Window, div, point, px, size,
};
use gpui_scrollbar::{
    Axis, ScrollbarInteraction, ScrollbarMountGeneration, ScrollbarOwnerId, ScrollbarOwnerKey,
    ScrollbarScrollState, ScrollbarState, ScrollbarStyle, ScrollbarVisibilityPolicy,
    render_scroll_handle_scrollbar, render_scrollbar,
};

pub(crate) fn key(owner: u64, mount: u64) -> ScrollbarOwnerKey {
    ScrollbarOwnerKey::new(
        ScrollbarOwnerId::new(owner),
        ScrollbarMountGeneration::new(mount),
    )
}

pub(crate) fn vertical_state(owner: ScrollbarOwnerKey, offset: Pixels) -> ScrollbarScrollState {
    ScrollbarScrollState {
        owner,
        viewport_bounds: Bounds::new(point(px(0.0), px(0.0)), size(px(240.0), px(240.0))),
        content_size: size(px(240.0), px(480.0)),
        scroll_offset: point(px(0.0), offset),
        page_distance: size(px(240.0), px(180.0)),
    }
}

pub(crate) fn interaction(
    current: Rc<Cell<ScrollbarScrollState>>,
    offsets: Rc<RefCell<Vec<Pixels>>>,
    pages: Rc<RefCell<Vec<(gpui_scrollbar::ScrollDirection, Pixels)>>>,
    started: Rc<Cell<usize>>,
    completed: Rc<Cell<usize>>,
) -> ScrollbarInteraction {
    let set_current = current.clone();
    ScrollbarInteraction::new(
        move || Some(current.get()),
        move |snapshot, offset| {
            let mut state = set_current.get();
            match snapshot.axis {
                Axis::Horizontal => state.scroll_offset.x = offset,
                Axis::Vertical => state.scroll_offset.y = offset,
            }
            set_current.set(state);
            offsets.borrow_mut().push(offset);
        },
        move |_, direction, distance| pages.borrow_mut().push((direction, distance)),
        move |_| started.set(started.get() + 1),
        move |_| completed.set(completed.get() + 1),
        |_, _, _| {},
    )
}

pub(crate) struct MountedScrollbarView {
    pub state: ScrollbarState,
    pub interaction: ScrollbarInteraction,
    pub axis: Axis,
}

pub(crate) struct MountedScrollHandleView {
    pub state: ScrollbarState,
    pub first_handle: ScrollHandle,
    pub second_handle: ScrollHandle,
    pub use_second_handle: bool,
}

impl Render for MountedScrollHandleView {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        let active_handle = if self.use_second_handle {
            &self.second_handle
        } else {
            &self.first_handle
        };
        div()
            .relative()
            .w(px(600.0))
            .h(px(240.0))
            .child(
                div()
                    .id("first-scroll-handle-viewport")
                    .absolute()
                    .left_0()
                    .top_0()
                    .w(px(240.0))
                    .h(px(240.0))
                    .overflow_y_scroll()
                    .track_scroll(&self.first_handle)
                    .child(div().w_full().h(px(480.0))),
            )
            .child(
                div()
                    .id("second-scroll-handle-viewport")
                    .absolute()
                    .left(px(300.0))
                    .top_0()
                    .w(px(240.0))
                    .h(px(240.0))
                    .overflow_y_scroll()
                    .track_scroll(&self.second_handle)
                    .child(div().w_full().h(px(480.0))),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .w(px(240.0))
                    .h(px(240.0))
                    .children(render_scroll_handle_scrollbar(
                        "mounted-scroll-handle-scrollbar",
                        self.state.current_owner().expect("mounted owner"),
                        self.state.clone(),
                        active_handle,
                        Axis::Vertical,
                        ScrollbarStyle::default(),
                        ScrollbarVisibilityPolicy::always_visible(),
                    )),
            )
    }
}

impl Render for MountedScrollbarView {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div().w(px(240.0)).h(px(240.0)).children(render_scrollbar(
            "mounted-scrollbar",
            self.state.clone(),
            self.axis,
            ScrollbarStyle::default(),
            ScrollbarVisibilityPolicy::always_visible(),
            self.interaction.clone(),
        ))
    }
}

pub(crate) fn has_active_drag(cx: &mut gpui::VisualTestContext) -> bool {
    cx.update(|_, cx| cx.has_active_drag())
}

pub(crate) fn assert_mounted_active_drag_termination(
    cx: &mut gpui::TestAppContext,
    owner: ScrollbarOwnerKey,
    terminate: impl FnOnce(&ScrollbarState, &mut Window, &mut gpui::App),
) {
    let current = Rc::new(Cell::new(vertical_state(owner, px(120.0))));
    let completed = Rc::new(Cell::new(0));
    let interaction = interaction(
        current,
        Default::default(),
        Default::default(),
        Default::default(),
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
    assert!(has_active_drag(cx));

    cx.update(|window, app| terminate(&state, window, app));

    assert!(!has_active_drag(cx));
    assert_eq!(completed.get(), 0);
}
