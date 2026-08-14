use std::rc::Rc;

use gpui::{App, Pixels, Point, ScrollHandle, Window, point, px};

use crate::{
    Axis, LaneClick, ScrollDirection, ScrollbarAxisHit, ScrollbarGeometrySnapshot,
    ScrollbarOwnerKey, ScrollbarScrollState, ScrollbarStyle, scrollbar_geometry_snapshot,
};

/// Callback invoked after keyed scrollbar chrome revalidates caller-owned state.
pub type ScrollbarOwnerUpdateCallback =
    Rc<dyn Fn(ScrollbarGeometrySnapshot, &mut Window, &mut App)>;

/// Caller-owned scroll callbacks used by rendered scrollbar chrome.
///
/// Scroll mutations receive the exact geometry snapshot validated immediately
/// before they are invoked. Owner updates receive a newly revalidated current
/// snapshot after that mutation.
#[derive(Clone)]
pub struct ScrollbarInteraction {
    identity: ScrollbarInteractionIdentity,
    state: Rc<dyn Fn() -> Option<ScrollbarScrollState>>,
    set_scroll_offset: Rc<dyn Fn(ScrollbarGeometrySnapshot, Pixels)>,
    page_scroll: Rc<dyn Fn(ScrollbarGeometrySnapshot, ScrollDirection, Pixels)>,
    drag_started: Rc<dyn Fn(ScrollbarGeometrySnapshot)>,
    drag_ended: Rc<dyn Fn(ScrollbarGeometrySnapshot)>,
    on_owner_update: ScrollbarOwnerUpdateCallback,
}

#[derive(Clone)]
enum ScrollbarInteractionIdentity {
    Callback(Rc<()>),
    ScrollHandle(ScrollHandle),
}

impl ScrollbarInteraction {
    pub(crate) fn is_same_instance(&self, other: &Self) -> bool {
        match (&self.identity, &other.identity) {
            (
                ScrollbarInteractionIdentity::Callback(left),
                ScrollbarInteractionIdentity::Callback(right),
            ) => Rc::ptr_eq(left, right),
            (
                ScrollbarInteractionIdentity::ScrollHandle(left),
                ScrollbarInteractionIdentity::ScrollHandle(right),
            ) => left.ptr_eq(right),
            _ => false,
        }
    }

    /// Creates an exact keyed interaction for a caller-owned scroll model.
    pub fn new(
        state: impl Fn() -> Option<ScrollbarScrollState> + 'static,
        set_scroll_offset: impl Fn(ScrollbarGeometrySnapshot, Pixels) + 'static,
        page_scroll: impl Fn(ScrollbarGeometrySnapshot, ScrollDirection, Pixels) + 'static,
        drag_started: impl Fn(ScrollbarGeometrySnapshot) + 'static,
        drag_ended: impl Fn(ScrollbarGeometrySnapshot) + 'static,
        on_owner_update: impl Fn(ScrollbarGeometrySnapshot, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            identity: ScrollbarInteractionIdentity::Callback(Rc::new(())),
            state: Rc::new(state),
            set_scroll_offset: Rc::new(set_scroll_offset),
            page_scroll: Rc::new(page_scroll),
            drag_started: Rc::new(drag_started),
            drag_ended: Rc::new(drag_ended),
            on_owner_update: Rc::new(on_owner_update),
        }
    }

    /// Creates an exact keyed interaction for an ordinary [`ScrollHandle`].
    #[must_use]
    pub fn for_scroll_handle(
        owner: ScrollbarOwnerKey,
        scroll_handle: ScrollHandle,
        axis: Axis,
    ) -> Self {
        Self::for_scroll_handle_with_owner_update(owner, scroll_handle, axis, |_, _, _| {})
    }

    /// Creates a keyed scroll-handle interaction with an owner update callback.
    #[must_use]
    pub fn for_scroll_handle_with_owner_update(
        owner: ScrollbarOwnerKey,
        scroll_handle: ScrollHandle,
        axis: Axis,
        on_owner_update: impl Fn(ScrollbarGeometrySnapshot, &mut Window, &mut App) + 'static,
    ) -> Self {
        let mut interaction = Self::new(
            {
                let scroll_handle = scroll_handle.clone();
                move || {
                    let viewport_bounds = scroll_handle.bounds();
                    let max_offset = scroll_handle.max_offset();
                    let offset = scroll_handle.offset();
                    Some(ScrollbarScrollState {
                        owner,
                        viewport_bounds,
                        content_size: viewport_bounds.size + max_offset,
                        scroll_offset: point(
                            (-offset.x).clamp(px(0.0), max_offset.width.max(px(0.0))),
                            (-offset.y).clamp(px(0.0), max_offset.height.max(px(0.0))),
                        ),
                        page_distance: viewport_bounds.size,
                    })
                }
            },
            {
                let scroll_handle = scroll_handle.clone();
                move |_, scroll_offset| {
                    let max_offset = axis_size(axis, scroll_handle.max_offset());
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
                move |_, direction, distance| {
                    let distance = distance.max(px(0.0));
                    let current_offset = scroll_handle.offset();
                    let current_scroll_offset = match axis {
                        Axis::Horizontal => -current_offset.x,
                        Axis::Vertical => -current_offset.y,
                    };
                    let max_offset = axis_size(axis, scroll_handle.max_offset());
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
            |_| {},
            |_| {},
            on_owner_update,
        );
        interaction.identity = ScrollbarInteractionIdentity::ScrollHandle(scroll_handle);
        interaction
    }

    /// Returns the current exact geometry record for an axis and style.
    #[must_use]
    pub fn current_snapshot(
        &self,
        axis: Axis,
        style: ScrollbarStyle,
    ) -> Option<ScrollbarGeometrySnapshot> {
        scrollbar_geometry_snapshot(axis, style.normalized().geometry, (self.state)()?)
    }

    pub(crate) fn snapshot_is_current(
        &self,
        expected: ScrollbarGeometrySnapshot,
        style: ScrollbarStyle,
    ) -> bool {
        self.current_snapshot(expected.axis, style) == Some(expected)
    }

    pub(crate) fn start_drag(
        &self,
        expected: ScrollbarGeometrySnapshot,
        style: ScrollbarStyle,
    ) -> bool {
        if !self.snapshot_is_current(expected, style) {
            return false;
        }
        (self.drag_started)(expected);
        true
    }

    pub(crate) fn end_drag_authorized(&self, snapshot: ScrollbarGeometrySnapshot) {
        (self.drag_ended)(snapshot);
    }

    pub(crate) fn current_owner_update(
        &self,
        expected: ScrollbarGeometrySnapshot,
        style: ScrollbarStyle,
    ) -> Option<ScrollbarGeometrySnapshot> {
        let Some(current) = self.current_snapshot(expected.axis, style) else {
            return None;
        };
        (current.owner == expected.owner).then_some(current)
    }

    pub(crate) fn owner_updated(
        &self,
        current: ScrollbarGeometrySnapshot,
        window: &mut Window,
        cx: &mut App,
    ) {
        (self.on_owner_update)(current, window, cx);
    }
}

/// Testable action derived from a pointer down inside one exact snapshot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollbarPointerDownAction {
    /// The pointer landed on the thumb and can begin dragging.
    StartDrag {
        /// Exact hit-tested geometry record.
        snapshot: ScrollbarGeometrySnapshot,
        /// Pointer offset from the thumb start, clamped to the thumb length.
        grab_offset: Pixels,
    },
    /// The pointer landed in a vertical lane outside the thumb.
    Page {
        /// Exact hit-tested geometry record.
        snapshot: ScrollbarGeometrySnapshot,
        /// Lane area that was clicked.
        lane: LaneClick,
        /// Page-scroll direction the owner should apply.
        direction: ScrollDirection,
        /// Positive page distance from the same snapshot.
        distance: Pixels,
    },
    /// The pointer does not produce scrollbar-owned scroll activity.
    Ignore,
}

/// Returns the scrollbar action for a pointer down on one exact snapshot.
#[must_use]
pub fn scrollbar_pointer_down_action(
    snapshot: ScrollbarGeometrySnapshot,
    pointer_position: Point<Pixels>,
) -> ScrollbarPointerDownAction {
    let axis_position = snapshot.axis_position(pointer_position);
    let hit = if axis_position < snapshot.thumb_bounds.start {
        ScrollbarAxisHit::LaneBeforeThumb
    } else if axis_position <= snapshot.thumb_bounds.end {
        ScrollbarAxisHit::Thumb
    } else {
        ScrollbarAxisHit::LaneAfterThumb
    };

    match hit {
        ScrollbarAxisHit::Thumb => ScrollbarPointerDownAction::StartDrag {
            snapshot,
            grab_offset: (axis_position - snapshot.thumb_bounds.start).clamp(
                px(0.0),
                snapshot.thumb_bounds.end - snapshot.thumb_bounds.start,
            ),
        },
        ScrollbarAxisHit::LaneBeforeThumb if snapshot.axis == Axis::Vertical => {
            page_action(snapshot, LaneClick::BeforeThumb)
        }
        ScrollbarAxisHit::LaneAfterThumb if snapshot.axis == Axis::Vertical => {
            page_action(snapshot, LaneClick::AfterThumb)
        }
        ScrollbarAxisHit::LaneBeforeThumb | ScrollbarAxisHit::LaneAfterThumb => {
            ScrollbarPointerDownAction::Ignore
        }
    }
}

fn page_action(snapshot: ScrollbarGeometrySnapshot, lane: LaneClick) -> ScrollbarPointerDownAction {
    ScrollbarPointerDownAction::Page {
        snapshot,
        lane,
        direction: lane.page_direction(),
        distance: snapshot.page_distance,
    }
}

/// Dispatches a pointer down only while its entire snapshot remains current.
///
/// A page request uses the hit-tested snapshot for its mutation, then obtains a
/// fresh current snapshot before returning the action used for the owner update.
/// This keeps the owner callback from observing pre-page geometry.
pub fn dispatch_scrollbar_pointer_down(
    state: &crate::ScrollbarState,
    style: ScrollbarStyle,
    interaction: &ScrollbarInteraction,
    snapshot: ScrollbarGeometrySnapshot,
    pointer_position: Point<Pixels>,
) -> ScrollbarPointerDownAction {
    if !state.has_current_owner(snapshot.owner) || !interaction.snapshot_is_current(snapshot, style)
    {
        return ScrollbarPointerDownAction::Ignore;
    }
    let action = scrollbar_pointer_down_action(snapshot, pointer_position);
    if let ScrollbarPointerDownAction::Page {
        snapshot,
        lane,
        direction,
        distance,
        ..
    } = action
    {
        if interaction.current_snapshot(snapshot.axis, style) != Some(snapshot)
            || !state.has_current_owner(snapshot.owner)
        {
            return ScrollbarPointerDownAction::Ignore;
        }
        (interaction.page_scroll)(snapshot, direction, distance);
        let Some(current) = interaction.current_owner_update(snapshot, style) else {
            return ScrollbarPointerDownAction::Ignore;
        };
        if !state.has_current_owner(current.owner) {
            return ScrollbarPointerDownAction::Ignore;
        }
        return ScrollbarPointerDownAction::Page {
            snapshot: current,
            lane,
            direction,
            distance,
        };
    }
    action
}

/// Maps a pointer position through one exact geometry snapshot.
#[must_use]
pub fn scrollbar_drag_scroll_offset(
    snapshot: ScrollbarGeometrySnapshot,
    pointer_position: Point<Pixels>,
    grab_offset: Pixels,
) -> Option<Pixels> {
    let thumb_length = snapshot.thumb_bounds.end - snapshot.thumb_bounds.start;
    let track_length = snapshot.track_bounds.end - snapshot.track_bounds.start;
    let thumb_travel = (track_length - thumb_length).max(px(0.0));
    if thumb_travel <= px(0.0) {
        return Some(px(0.0));
    }
    let desired_thumb_offset =
        (snapshot.axis_position(pointer_position) - snapshot.track_bounds.start - grab_offset)
            .clamp(px(0.0), thumb_travel);
    Some(snapshot.overflow_length() * (desired_thumb_offset / thumb_travel))
}

/// Dispatches a drag update under the current snapshot of one mounted owner.
///
/// The current snapshot is obtained and then compared again immediately before
/// the mutation callback. A geometry change during either lookup rejects the
/// update rather than mixing records.
pub(crate) fn dispatch_scrollbar_drag(
    snapshot: ScrollbarGeometrySnapshot,
    interaction: &ScrollbarInteraction,
    next_offset: Pixels,
) -> Option<(ScrollbarGeometrySnapshot, Pixels)> {
    (interaction.set_scroll_offset)(snapshot, next_offset);
    Some((snapshot, next_offset))
}

fn axis_size(axis: Axis, size: gpui::Size<Pixels>) -> Pixels {
    match axis {
        Axis::Horizontal => size.width,
        Axis::Vertical => size.height,
    }
    .max(px(0.0))
}
