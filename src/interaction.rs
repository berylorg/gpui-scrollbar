use std::rc::Rc;

use gpui::{App, Bounds, Pixels, Point, ScrollHandle, Size, Window, point, px};

use crate::{
    Axis, LaneClick, ScrollDirection, ScrollbarAxisHit, ScrollbarStyle,
    classify_scrollbar_axis_hit, scroll_offset_from_thumb_drag, scrollbar_metrics,
    scrollbar_thumb_grab_offset,
};

/// Current scroll geometry supplied by the scrollable owner.
///
/// `scroll_offset` is a positive visible scroll distance. The helper for
/// [`ScrollHandle`] converts from GPUI's negative content offset convention.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarScrollState {
    /// Viewport bounds in window coordinates.
    pub viewport_bounds: Bounds<Pixels>,
    /// Maximum positive scroll distance for each axis.
    pub max_offset: Size<Pixels>,
    /// Current positive visible scroll distance for each axis.
    pub scroll_offset: Point<Pixels>,
}

/// Callback invoked when scrollbar chrome activity should wake application UI.
pub type ScrollbarActivityCallback = Rc<dyn Fn(&mut Window, &mut App)>;

/// Caller-owned scroll callbacks used by rendered scrollbar chrome.
#[derive(Clone)]
pub struct ScrollbarInteraction {
    state: Rc<dyn Fn() -> Option<ScrollbarScrollState>>,
    set_scroll_offset: Rc<dyn Fn(Pixels)>,
    page_scroll: Rc<dyn Fn(ScrollDirection, Pixels)>,
    drag_started: Rc<dyn Fn()>,
    drag_ended: Rc<dyn Fn()>,
    on_activity: Option<ScrollbarActivityCallback>,
}

impl ScrollbarInteraction {
    /// Creates a callback-backed scrollbar interaction for caller-owned scroll models.
    pub fn new(
        state: impl Fn() -> Option<ScrollbarScrollState> + 'static,
        set_scroll_offset: impl Fn(Pixels) + 'static,
        page_scroll: impl Fn(ScrollDirection, Pixels) + 'static,
        drag_started: impl Fn() + 'static,
        drag_ended: impl Fn() + 'static,
        on_activity: Option<ScrollbarActivityCallback>,
    ) -> Self {
        Self {
            state: Rc::new(state),
            set_scroll_offset: Rc::new(set_scroll_offset),
            page_scroll: Rc::new(page_scroll),
            drag_started: Rc::new(drag_started),
            drag_ended: Rc::new(drag_ended),
            on_activity,
        }
    }

    /// Creates a scrollbar interaction for an ordinary [`ScrollHandle`].
    #[must_use]
    pub fn for_scroll_handle(
        scroll_handle: ScrollHandle,
        axis: Axis,
        on_activity: Option<ScrollbarActivityCallback>,
    ) -> Self {
        Self::new(
            {
                let scroll_handle = scroll_handle.clone();
                move || {
                    let max_offset = scroll_handle.max_offset();
                    let offset = scroll_handle.offset();
                    Some(ScrollbarScrollState {
                        viewport_bounds: scroll_handle.bounds(),
                        max_offset,
                        scroll_offset: point(
                            (-offset.x).clamp(px(0.0), max_offset.width.max(px(0.0))),
                            (-offset.y).clamp(px(0.0), max_offset.height.max(px(0.0))),
                        ),
                    })
                }
            },
            {
                let scroll_handle = scroll_handle.clone();
                move |scroll_offset| {
                    let max_offset = scrollbar_axis_max_offset(axis, scroll_handle.max_offset());
                    let scroll_offset = scroll_offset.clamp(px(0.0), max_offset);
                    let current_offset = scroll_handle.offset();
                    scroll_handle.set_offset(match axis {
                        Axis::Horizontal => point(-scroll_offset, current_offset.y),
                        Axis::Vertical => point(current_offset.x, -scroll_offset),
                    });
                }
            },
            {
                let scroll_handle = scroll_handle.clone();
                move |direction, distance| {
                    let distance = distance.max(px(0.0));
                    let current_offset = scroll_handle.offset();
                    let current_scroll_offset = match axis {
                        Axis::Horizontal => -current_offset.x,
                        Axis::Vertical => -current_offset.y,
                    };
                    let max_offset = scrollbar_axis_max_offset(axis, scroll_handle.max_offset());
                    let next_scroll_offset = match direction {
                        ScrollDirection::Backward => current_scroll_offset - distance,
                        ScrollDirection::Forward => current_scroll_offset + distance,
                    }
                    .clamp(px(0.0), max_offset);
                    scroll_handle.set_offset(match axis {
                        Axis::Horizontal => point(-next_scroll_offset, current_offset.y),
                        Axis::Vertical => point(current_offset.x, -next_scroll_offset),
                    });
                }
            },
            || {},
            || {},
            on_activity,
        )
    }

    /// Returns the current caller-owned scroll state.
    #[must_use]
    pub fn current_state(&self) -> Option<ScrollbarScrollState> {
        (self.state)()
    }

    /// Sets the positive scroll offset along the scrollbar axis.
    pub fn set_scroll_offset(&self, offset: Pixels) {
        (self.set_scroll_offset)(offset);
    }

    /// Requests one page scroll in the given direction.
    pub fn page_scroll(&self, direction: ScrollDirection, distance: Pixels) {
        (self.page_scroll)(direction, distance);
    }

    /// Notifies the owner that thumb dragging started.
    pub fn drag_started(&self) {
        (self.drag_started)();
    }

    /// Notifies the owner that thumb dragging ended.
    pub fn drag_ended(&self) {
        (self.drag_ended)();
    }

    /// Records scrollbar chrome activity and refreshes the window.
    pub fn record_activity(&self, window: &mut Window, cx: &mut App) {
        if let Some(on_activity) = self.on_activity.as_ref() {
            on_activity(window, cx);
        }
        window.refresh();
    }
}

/// Testable action derived from a pointer down inside the scrollbar lane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollbarPointerDownAction {
    /// The pointer landed on the thumb and can begin dragging.
    StartDrag {
        /// Pointer offset from the thumb start, clamped to the thumb length.
        grab_offset: Pixels,
    },
    /// The pointer landed in a vertical lane outside the thumb.
    Page {
        /// Lane area that was clicked.
        lane: LaneClick,
        /// Page-scroll direction the owner should apply.
        direction: ScrollDirection,
        /// Page-scroll distance, equal to the viewport length on the axis.
        distance: Pixels,
    },
    /// The pointer does not produce scrollbar-owned scroll activity.
    Ignore,
}

/// Returns the scrollbar action for a pointer down without invoking callbacks.
#[must_use]
pub fn scrollbar_pointer_down_action(
    axis: Axis,
    style: ScrollbarStyle,
    state: ScrollbarScrollState,
    pointer_position: Point<Pixels>,
) -> ScrollbarPointerDownAction {
    let style = style.normalized();
    let viewport_length = scrollbar_axis_length(axis, state.viewport_bounds.size);
    let overflow_length = scrollbar_axis_max_offset(axis, state.max_offset);
    let scroll_offset =
        scrollbar_axis_scroll_offset(axis, state.scroll_offset).clamp(px(0.0), overflow_length);
    let Some(metrics) = scrollbar_metrics(
        style.geometry,
        viewport_length,
        overflow_length,
        scroll_offset,
    ) else {
        return ScrollbarPointerDownAction::Ignore;
    };
    let axis_position =
        scrollbar_axis_position_in_viewport(axis, pointer_position, state.viewport_bounds);

    match classify_scrollbar_axis_hit(style.geometry, viewport_length, metrics, axis_position) {
        Some(ScrollbarAxisHit::Thumb) => {
            scrollbar_thumb_grab_offset(style.geometry, viewport_length, metrics, axis_position)
                .map(|grab_offset| ScrollbarPointerDownAction::StartDrag { grab_offset })
                .unwrap_or(ScrollbarPointerDownAction::Ignore)
        }
        Some(ScrollbarAxisHit::LaneBeforeThumb) if axis == Axis::Vertical => {
            let lane = LaneClick::BeforeThumb;
            ScrollbarPointerDownAction::Page {
                lane,
                direction: lane.page_direction(),
                distance: viewport_length,
            }
        }
        Some(ScrollbarAxisHit::LaneAfterThumb) if axis == Axis::Vertical => {
            let lane = LaneClick::AfterThumb;
            ScrollbarPointerDownAction::Page {
                lane,
                direction: lane.page_direction(),
                distance: viewport_length,
            }
        }
        Some(ScrollbarAxisHit::LaneBeforeThumb | ScrollbarAxisHit::LaneAfterThumb) | None => {
            ScrollbarPointerDownAction::Ignore
        }
    }
}

/// Dispatches a pointer down through the interaction callbacks when appropriate.
pub fn dispatch_scrollbar_pointer_down(
    axis: Axis,
    style: ScrollbarStyle,
    interaction: &ScrollbarInteraction,
    pointer_position: Point<Pixels>,
) -> ScrollbarPointerDownAction {
    let Some(state) = interaction.current_state() else {
        return ScrollbarPointerDownAction::Ignore;
    };
    let action = scrollbar_pointer_down_action(axis, style, state, pointer_position);
    if let ScrollbarPointerDownAction::Page {
        direction,
        distance,
        ..
    } = action
    {
        interaction.page_scroll(direction, distance);
    }
    action
}

/// Maps an active thumb drag to a scroll offset without invoking callbacks.
#[must_use]
pub fn scrollbar_drag_scroll_offset(
    axis: Axis,
    style: ScrollbarStyle,
    state: ScrollbarScrollState,
    pointer_position: Point<Pixels>,
    grab_offset: Pixels,
) -> Option<Pixels> {
    let style = style.normalized();
    let viewport_length = scrollbar_axis_length(axis, state.viewport_bounds.size);
    let overflow_length = scrollbar_axis_max_offset(axis, state.max_offset);
    let scroll_offset =
        scrollbar_axis_scroll_offset(axis, state.scroll_offset).clamp(px(0.0), overflow_length);
    let metrics = scrollbar_metrics(
        style.geometry,
        viewport_length,
        overflow_length,
        scroll_offset,
    )?;
    let pointer_axis_position =
        scrollbar_axis_position_in_viewport(axis, pointer_position, state.viewport_bounds);
    scroll_offset_from_thumb_drag(
        style.geometry,
        viewport_length,
        overflow_length,
        metrics,
        pointer_axis_position,
        grab_offset,
    )
}

/// Dispatches an active thumb drag through the interaction callbacks.
pub fn dispatch_scrollbar_drag(
    axis: Axis,
    style: ScrollbarStyle,
    interaction: &ScrollbarInteraction,
    pointer_position: Point<Pixels>,
    grab_offset: Pixels,
) -> Option<Pixels> {
    let state = interaction.current_state()?;
    let next_offset =
        scrollbar_drag_scroll_offset(axis, style, state, pointer_position, grab_offset)?;
    interaction.set_scroll_offset(next_offset);
    Some(next_offset)
}

pub(crate) fn scrollbar_axis_length(axis: Axis, size: Size<Pixels>) -> Pixels {
    match axis {
        Axis::Horizontal => size.width,
        Axis::Vertical => size.height,
    }
}

pub(crate) fn scrollbar_axis_max_offset(axis: Axis, max_offset: Size<Pixels>) -> Pixels {
    match axis {
        Axis::Horizontal => max_offset.width,
        Axis::Vertical => max_offset.height,
    }
    .max(px(0.0))
}

pub(crate) fn scrollbar_axis_scroll_offset(axis: Axis, scroll_offset: Point<Pixels>) -> Pixels {
    match axis {
        Axis::Horizontal => scroll_offset.x,
        Axis::Vertical => scroll_offset.y,
    }
}

pub(crate) fn scrollbar_axis_position_in_viewport(
    axis: Axis,
    position: Point<Pixels>,
    viewport_bounds: Bounds<Pixels>,
) -> Pixels {
    match axis {
        Axis::Horizontal => position.x - viewport_bounds.left(),
        Axis::Vertical => position.y - viewport_bounds.top(),
    }
}
