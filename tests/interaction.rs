use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui::{Bounds, Pixels, point, px, size};
use gpui_scrollbar::{
    Axis, LaneClick, ScrollDirection, ScrollbarPointerDownAction, ScrollbarScrollState,
    ScrollbarStyle, dispatch_scrollbar_drag, dispatch_scrollbar_pointer_down, scrollbar_metrics,
    scrollbar_pointer_down_action, scrollbar_track_geometry,
};

fn vertical_state(scroll_offset: Pixels) -> ScrollbarScrollState {
    ScrollbarScrollState {
        viewport_bounds: Bounds::new(point(px(0.0), px(0.0)), size(px(240.0), px(240.0))),
        max_offset: size(px(0.0), px(240.0)),
        scroll_offset: point(px(0.0), scroll_offset),
    }
}

fn horizontal_state(scroll_offset: Pixels) -> ScrollbarScrollState {
    ScrollbarScrollState {
        viewport_bounds: Bounds::new(point(px(0.0), px(0.0)), size(px(240.0), px(240.0))),
        max_offset: size(px(240.0), px(0.0)),
        scroll_offset: point(scroll_offset, px(0.0)),
    }
}

#[test]
fn pointer_down_on_thumb_starts_drag_with_grab_offset() {
    let style = ScrollbarStyle::default();
    let state = vertical_state(px(120.0));
    let metrics = scrollbar_metrics(style.geometry, px(240.0), px(240.0), px(120.0))
        .expect("overflow should produce metrics");
    let geometry = scrollbar_track_geometry(style.geometry, px(240.0), metrics)
        .expect("metrics should produce track geometry");

    let action = scrollbar_pointer_down_action(
        Axis::Vertical,
        style,
        state,
        point(px(236.0), geometry.thumb_start + px(10.0)),
    );

    assert_eq!(
        action,
        ScrollbarPointerDownAction::StartDrag {
            grab_offset: px(10.0)
        }
    );
}

#[test]
fn vertical_lane_pointer_down_dispatches_one_page() {
    let style = ScrollbarStyle::default();
    let state = vertical_state(px(120.0));
    let metrics = scrollbar_metrics(style.geometry, px(240.0), px(240.0), px(120.0))
        .expect("overflow should produce metrics");
    let geometry = scrollbar_track_geometry(style.geometry, px(240.0), metrics)
        .expect("metrics should produce track geometry");
    let pages = Rc::new(RefCell::new(Vec::new()));
    let interaction = gpui_scrollbar::ScrollbarInteraction::new(
        move || Some(state),
        |_| {},
        {
            let pages = pages.clone();
            move |direction, distance| pages.borrow_mut().push((direction, distance))
        },
        || {},
        || {},
        |_, _| {},
    );

    let before = dispatch_scrollbar_pointer_down(
        Axis::Vertical,
        style,
        &interaction,
        point(px(236.0), geometry.thumb_start - px(1.0)),
    );
    let after = dispatch_scrollbar_pointer_down(
        Axis::Vertical,
        style,
        &interaction,
        point(px(236.0), geometry.thumb_end + px(1.0)),
    );

    assert_eq!(
        before,
        ScrollbarPointerDownAction::Page {
            lane: LaneClick::BeforeThumb,
            direction: ScrollDirection::Backward,
            distance: px(240.0)
        }
    );
    assert_eq!(
        after,
        ScrollbarPointerDownAction::Page {
            lane: LaneClick::AfterThumb,
            direction: ScrollDirection::Forward,
            distance: px(240.0)
        }
    );
    assert_eq!(
        pages.borrow().as_slice(),
        &[
            (ScrollDirection::Backward, px(240.0)),
            (ScrollDirection::Forward, px(240.0))
        ]
    );
}

#[test]
fn horizontal_lane_pointer_down_is_ignored() {
    let style = ScrollbarStyle::default();
    let state = horizontal_state(px(120.0));
    let metrics = scrollbar_metrics(style.geometry, px(240.0), px(240.0), px(120.0))
        .expect("overflow should produce metrics");
    let geometry = scrollbar_track_geometry(style.geometry, px(240.0), metrics)
        .expect("metrics should produce track geometry");
    let pages = Rc::new(Cell::new(0));
    let interaction = gpui_scrollbar::ScrollbarInteraction::new(
        move || Some(state),
        |_| {},
        {
            let pages = pages.clone();
            move |_, _| pages.set(pages.get() + 1)
        },
        || {},
        || {},
        |_, _| {},
    );

    let before = dispatch_scrollbar_pointer_down(
        Axis::Horizontal,
        style,
        &interaction,
        point(geometry.thumb_start - px(1.0), px(236.0)),
    );
    let after = dispatch_scrollbar_pointer_down(
        Axis::Horizontal,
        style,
        &interaction,
        point(geometry.thumb_end + px(1.0), px(236.0)),
    );

    assert_eq!(before, ScrollbarPointerDownAction::Ignore);
    assert_eq!(after, ScrollbarPointerDownAction::Ignore);
    assert_eq!(pages.get(), 0);
}

#[test]
fn drag_dispatch_maps_pointer_to_owner_callback_offset() {
    let style = ScrollbarStyle::default();
    let state = vertical_state(px(0.0));
    let metrics = scrollbar_metrics(style.geometry, px(240.0), px(240.0), px(0.0))
        .expect("overflow should produce metrics");
    let geometry = scrollbar_track_geometry(style.geometry, px(240.0), metrics)
        .expect("metrics should produce track geometry");
    let offsets = Rc::new(RefCell::new(Vec::new()));
    let interaction = gpui_scrollbar::ScrollbarInteraction::new(
        move || Some(state),
        {
            let offsets = offsets.clone();
            move |offset| offsets.borrow_mut().push(offset)
        },
        |_, _| {},
        || {},
        || {},
        |_, _| {},
    );

    let next = dispatch_scrollbar_drag(
        Axis::Vertical,
        style,
        &interaction,
        point(px(236.0), geometry.track_start + px(57.0) + px(10.0)),
        px(10.0),
    );

    assert_eq!(next, Some(px(120.0)));
    assert_eq!(offsets.borrow().as_slice(), &[px(120.0)]);
}
