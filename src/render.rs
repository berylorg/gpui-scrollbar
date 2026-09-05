use std::{cell::Cell, time::Instant};

use gpui::{
    AnyElement, App, Context, ElementId, MouseButton, Pixels, Point, Render, ScrollHandle, Window,
    canvas, div, prelude::*, px, rgb,
};

use crate::lifecycle::{ScrollbarDragInstance, ScrollbarRenderIdentity};
use crate::{
    Axis, ScrollbarGeometrySnapshot, ScrollbarInteraction, ScrollbarOwnerKey,
    ScrollbarPointerDownAction, ScrollbarState, ScrollbarStyle, ScrollbarVisibilityKey,
    ScrollbarVisibilityPolicy, dispatch_scrollbar_pointer_down,
};

/// Renders an exact keyed interactive scrollbar backed by a [`ScrollHandle`].
#[must_use]
pub fn render_scroll_handle_scrollbar(
    id: impl Into<ElementId>,
    owner: ScrollbarOwnerKey,
    state: ScrollbarState,
    scroll_handle: &ScrollHandle,
    axis: Axis,
    style: ScrollbarStyle,
    visibility: ScrollbarVisibilityPolicy,
) -> Option<AnyElement> {
    let interaction = ScrollbarInteraction::for_scroll_handle(owner, scroll_handle.clone(), axis);
    render_scrollbar(id, state, axis, style, visibility, interaction)
}

/// Renders an exact keyed scrollbar backed by caller-owned callbacks.
#[must_use]
pub fn render_scrollbar(
    id: impl Into<ElementId>,
    state: ScrollbarState,
    axis: Axis,
    style: ScrollbarStyle,
    visibility: ScrollbarVisibilityPolicy,
    interaction: ScrollbarInteraction,
) -> Option<AnyElement> {
    let style = style.normalized();
    let Some(snapshot) = interaction.current_snapshot(axis, style) else {
        state.retire_render_constructor();
        return None;
    };
    if !state.has_current_owner(snapshot.owner) {
        state.retire_render_constructor();
        return None;
    }
    let render_identity = state.claim_render_identity(snapshot, style, &interaction)?;
    let frame_key = visibility.current_frame_key(snapshot.owner, true, Instant::now());
    let opacity = match visibility.opacity_for_owner_at(snapshot.owner, true, Instant::now()) {
        Some(opacity) => opacity,
        None if frame_key.is_some() => 0.0,
        None => {
            state.retire_render_constructor();
            return None;
        }
    };
    Some(render_scrollbar_with_snapshot(
        id.into(),
        state,
        render_identity,
        snapshot,
        style,
        opacity,
        frame_key,
        visibility,
        interaction,
    ))
}

fn render_scrollbar_with_snapshot(
    id: ElementId,
    state: ScrollbarState,
    render_identity: ScrollbarRenderIdentity,
    snapshot: ScrollbarGeometrySnapshot,
    style: ScrollbarStyle,
    opacity: f32,
    frame_key: Option<ScrollbarVisibilityKey>,
    visibility: ScrollbarVisibilityPolicy,
    interaction: ScrollbarInteraction,
) -> AnyElement {
    let thumb = render_interactive_scrollbar_thumb(
        (id.clone(), "thumb").into(),
        state.clone(),
        render_identity,
        snapshot,
        style,
        opacity,
        visibility.clone(),
        interaction.clone(),
    );
    let mut lane = match snapshot.axis {
        Axis::Horizontal => div()
            .absolute()
            .left_0()
            .bottom_0()
            .w_full()
            .h(style.hit_lane_thickness),
        Axis::Vertical => div()
            .absolute()
            .top_0()
            .right_0()
            .h_full()
            .w(style.hit_lane_thickness),
    }
    .id(id)
    .child(thumb);
    if let Some(frame_key) = frame_key {
        lane = lane.child(render_animation_frame_driver(visibility.clone(), frame_key));
    }
    lane = lane.on_mouse_down(MouseButton::Left, {
        let state = state.clone();
        let interaction = interaction.clone();
        let visibility = visibility.clone();
        move |event, window, cx| {
            handle_lane_mouse_down(
                &state,
                snapshot,
                style,
                &interaction,
                &visibility,
                event.position,
                window,
                cx,
            );
            cx.stop_propagation();
        }
    });
    lane.into_any_element()
}

fn render_animation_frame_driver(
    visibility: ScrollbarVisibilityPolicy,
    expected: ScrollbarVisibilityKey,
) -> AnyElement {
    canvas(
        |_, _, _| (),
        move |_, _, window, _| {
            #[cfg(feature = "test-support")]
            visibility.record_frame_driver(expected);
            visibility.request_animation_frame_for_key(expected, window);
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .w(px(0.0))
    .h(px(0.0))
    .into_any_element()
}

/// Renders a non-interactive scrollbar thumb.
#[must_use]
pub fn render_scrollbar_thumb(
    axis: Axis,
    metrics: crate::ScrollbarMetrics,
    style: ScrollbarStyle,
    opacity: f32,
) -> AnyElement {
    let style = style.normalized();
    match axis {
        Axis::Horizontal => div()
            .absolute()
            .left(style.geometry.track_inset + metrics.thumb_offset)
            .bottom(style.geometry.track_inset)
            .h(style.thickness)
            .w(metrics.thumb_length)
            .rounded_full()
            .bg(rgb(style.thumb_color))
            .opacity(opacity)
            .into_any_element(),
        Axis::Vertical => div()
            .absolute()
            .top(style.geometry.track_inset + metrics.thumb_offset)
            .right(style.geometry.track_inset)
            .w(style.thickness)
            .h(metrics.thumb_length)
            .rounded_full()
            .bg(rgb(style.thumb_color))
            .opacity(opacity)
            .into_any_element(),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_interactive_scrollbar_thumb(
    id: ElementId,
    state: ScrollbarState,
    render_identity: ScrollbarRenderIdentity,
    snapshot: ScrollbarGeometrySnapshot,
    style: ScrollbarStyle,
    opacity: f32,
    visibility: ScrollbarVisibilityPolicy,
    interaction: ScrollbarInteraction,
) -> AnyElement {
    let style = style.normalized();
    let metrics = snapshot.metrics();
    let pending_instance = state.pending_drag_instance(render_identity);
    let active_instance = state.active_drag_instance();
    let drag_instance = state.next_drag_instance();
    let thumb = match snapshot.axis {
        Axis::Horizontal => div()
            .absolute()
            .left(style.geometry.track_inset + metrics.thumb_offset)
            .bottom(style.geometry.track_inset)
            .h(style.thickness)
            .w(metrics.thumb_length),
        Axis::Vertical => div()
            .absolute()
            .top(style.geometry.track_inset + metrics.thumb_offset)
            .right(style.geometry.track_inset)
            .w(style.thickness)
            .h(metrics.thumb_length),
    };
    let drag_value = ScrollbarDragValue {
        state: state.clone(),
        render_identity,
        pending_instance,
        drag_instance,
        snapshot,
        interaction: interaction.clone(),
        visibility: visibility.clone(),
        owns_active_drag: Cell::new(false),
    };
    thumb
        .id(id)
        .rounded_full()
        .bg(rgb(style.thumb_color))
        .opacity(opacity)
        .on_mouse_down(MouseButton::Left, {
            let state = state.clone();
            let interaction = interaction.clone();
            move |event, _, cx| {
                if let ScrollbarPointerDownAction::StartDrag {
                    snapshot,
                    grab_offset,
                } = dispatch_scrollbar_pointer_down(
                    &state,
                    style,
                    &interaction,
                    snapshot,
                    event.position,
                ) {
                    state.begin_pending_drag(
                        render_identity,
                        pending_instance,
                        style,
                        snapshot,
                        grab_offset,
                        &interaction,
                    );
                }
                cx.stop_propagation();
            }
        })
        .on_drag(drag_value, |drag: &ScrollbarDragValue, _, window, cx| {
            if !drag.start(window, cx) {
                window.defer(cx, |window, cx| {
                    cx.stop_active_drag(window);
                });
            }
            cx.new(|_| ScrollbarDragPreview)
        })
        .on_drag_move::<ScrollbarDragValue>(
            move |event: &gpui::DragMoveEvent<ScrollbarDragValue>, window, cx| {
                let (state, visibility, drag_instance) = {
                    let drag = event.drag(cx);
                    (
                        drag.state.clone(),
                        drag.visibility.clone(),
                        drag.drag_instance,
                    )
                };
                if let Some((current, _)) =
                    state.update_drag_instance(drag_instance, event.event.position)
                {
                    state.owner_updated_instance(drag_instance, window, cx);
                    visibility.record_direct_activity(current.owner, window, cx);
                } else {
                    cx.stop_active_drag(window);
                }
                cx.stop_propagation();
            },
        )
        .on_mouse_up(MouseButton::Left, {
            let state = state.clone();
            let visibility = visibility.clone();
            move |_, window, cx| {
                finish_pointer_lifecycle(
                    &state,
                    &visibility,
                    snapshot.owner,
                    pending_instance,
                    active_instance,
                    window,
                    cx,
                );
            }
        })
        .on_mouse_up_out(MouseButton::Left, {
            let state = state.clone();
            let visibility = visibility.clone();
            move |_, window, cx| {
                finish_pointer_lifecycle(
                    &state,
                    &visibility,
                    snapshot.owner,
                    pending_instance,
                    active_instance,
                    window,
                    cx,
                );
            }
        })
        .into_any_element()
}

struct ScrollbarDragValue {
    state: ScrollbarState,
    render_identity: ScrollbarRenderIdentity,
    pending_instance: ScrollbarDragInstance,
    drag_instance: ScrollbarDragInstance,
    snapshot: ScrollbarGeometrySnapshot,
    interaction: ScrollbarInteraction,
    visibility: ScrollbarVisibilityPolicy,
    owns_active_drag: Cell<bool>,
}

impl ScrollbarDragValue {
    fn start(&self, window: &mut Window, cx: &mut App) -> bool {
        if self.state.start_pending_drag(
            self.render_identity,
            self.pending_instance,
            self.drag_instance,
            &self.interaction,
        ) {
            self.owns_active_drag.set(true);
            self.visibility
                .begin_direct_interaction(self.snapshot.owner, window, cx);
            true
        } else {
            false
        }
    }
}

impl Drop for ScrollbarDragValue {
    fn drop(&mut self) {
        if self.owns_active_drag.get() {
            self.state.cancel_drag_instance(self.drag_instance);
        }
    }
}

struct ScrollbarDragPreview;

impl Render for ScrollbarDragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(0.0)).h(px(0.0))
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_lane_mouse_down(
    state: &ScrollbarState,
    snapshot: ScrollbarGeometrySnapshot,
    style: ScrollbarStyle,
    interaction: &ScrollbarInteraction,
    visibility: &ScrollbarVisibilityPolicy,
    position: Point<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    match dispatch_scrollbar_pointer_down(state, style, interaction, snapshot, position) {
        ScrollbarPointerDownAction::Page { snapshot, .. }
            if state.has_current_owner(snapshot.owner) =>
        {
            interaction.owner_updated(snapshot, window, cx);
            visibility.record_direct_activity(snapshot.owner, window, cx);
        }
        ScrollbarPointerDownAction::StartDrag { .. }
        | ScrollbarPointerDownAction::Page { .. }
        | ScrollbarPointerDownAction::Ignore => {}
    }
}

fn finish_pointer_lifecycle(
    state: &ScrollbarState,
    visibility: &ScrollbarVisibilityPolicy,
    owner: ScrollbarOwnerKey,
    pending_instance: ScrollbarDragInstance,
    active_instance: Option<ScrollbarDragInstance>,
    window: &mut Window,
    cx: &mut App,
) {
    state.cancel_pending_drag(pending_instance);
    let completion = match active_instance {
        Some(active_instance) => state.settle_drag_instance(active_instance),
        None => state.settle_drag_from_pending(Some(pending_instance)),
    };
    if completion.finishes_pointer() {
        visibility.end_direct_interaction(owner, window, cx);
    }
}
