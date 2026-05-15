use std::{cell::RefCell, rc::Rc};

use gpui::{
    AnyElement, App, Context, ElementId, MouseButton, Pixels, Point, Render, ScrollHandle, Window,
    div, prelude::*, px, rgb,
};

use crate::interaction::{
    ScrollbarInteraction, ScrollbarPointerDownAction, dispatch_scrollbar_drag,
    dispatch_scrollbar_pointer_down, scrollbar_axis_length, scrollbar_axis_max_offset,
    scrollbar_axis_scroll_offset,
};
use crate::{Axis, ScrollbarActivityCallback, ScrollbarMetrics, ScrollbarStyle, scrollbar_metrics};

/// Renders an interactive scrollbar backed by an ordinary [`ScrollHandle`].
#[must_use]
pub fn render_scroll_handle_scrollbar(
    id: impl Into<ElementId>,
    scroll_handle: &ScrollHandle,
    axis: Axis,
    style: ScrollbarStyle,
    opacity: f32,
    on_activity: Option<ScrollbarActivityCallback>,
) -> Option<AnyElement> {
    let interaction =
        ScrollbarInteraction::for_scroll_handle(scroll_handle.clone(), axis, on_activity);
    render_scrollbar(id, axis, style, opacity, interaction)
}

/// Renders an interactive scrollbar backed by caller-owned scroll callbacks.
#[must_use]
pub fn render_scrollbar(
    id: impl Into<ElementId>,
    axis: Axis,
    style: ScrollbarStyle,
    opacity: f32,
    interaction: ScrollbarInteraction,
) -> Option<AnyElement> {
    if opacity <= 0.0 {
        return None;
    }

    let style = style.normalized();
    let state = interaction.current_state()?;
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

    Some(render_scrollbar_with_metrics(
        id.into(),
        axis,
        metrics,
        style,
        opacity,
        interaction,
    ))
}

fn render_scrollbar_with_metrics(
    id: ElementId,
    axis: Axis,
    metrics: ScrollbarMetrics,
    style: ScrollbarStyle,
    opacity: f32,
    interaction: ScrollbarInteraction,
) -> AnyElement {
    let pending_drag = Rc::new(RefCell::new(None));
    let active_drag = Rc::new(RefCell::new(None));
    let thumb = render_scrollbar_thumb(axis, metrics, style, opacity);
    let mut lane = match axis {
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

    let drag_value = ScrollbarDragValue {
        axis,
        style,
        interaction: interaction.clone(),
        pending_drag: pending_drag.clone(),
        active_drag: active_drag.clone(),
    };
    lane = lane
        .on_mouse_down(MouseButton::Left, {
            let interaction = interaction.clone();
            let pending_drag = pending_drag.clone();
            move |event, window, cx| {
                handle_scrollbar_mouse_down(
                    axis,
                    style,
                    &interaction,
                    &pending_drag,
                    event.position,
                    window,
                    cx,
                );
                cx.stop_propagation();
            }
        })
        .on_drag(drag_value, |drag: &ScrollbarDragValue, _, window, cx| {
            drag.start(window, cx);
            cx.new(|_| ScrollbarDragPreview)
        })
        .on_drag_move::<ScrollbarDragValue>(
            move |event: &gpui::DragMoveEvent<ScrollbarDragValue>, window, cx| {
                let (axis, style, interaction, active_drag) = {
                    let drag = event.drag(cx);
                    (
                        drag.axis,
                        drag.style,
                        drag.interaction.clone(),
                        drag.active_drag.clone(),
                    )
                };
                update_scrollbar_drag(
                    axis,
                    style,
                    &interaction,
                    &active_drag,
                    event.event.position,
                    window,
                    cx,
                );
                cx.stop_propagation();
            },
        );

    lane.into_any_element()
}

/// Renders a non-interactive scrollbar thumb.
#[must_use]
pub fn render_scrollbar_thumb(
    axis: Axis,
    metrics: ScrollbarMetrics,
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

#[derive(Clone, Copy)]
enum PendingScrollbarDrag {
    Thumb { grab_offset: Pixels },
    Ignore,
}

#[derive(Clone, Copy)]
struct ScrollbarActiveDrag {
    grab_offset: Pixels,
}

struct ScrollbarDragValue {
    axis: Axis,
    style: ScrollbarStyle,
    interaction: ScrollbarInteraction,
    pending_drag: Rc<RefCell<Option<PendingScrollbarDrag>>>,
    active_drag: Rc<RefCell<Option<ScrollbarActiveDrag>>>,
}

impl ScrollbarDragValue {
    fn start(&self, window: &mut Window, cx: &mut App) {
        let pending = self.pending_drag.borrow_mut().take();
        let grab_offset = match pending {
            Some(PendingScrollbarDrag::Thumb { grab_offset }) => Some(grab_offset),
            Some(PendingScrollbarDrag::Ignore) => None,
            None => self.thumb_grab_offset_at(window.mouse_position()),
        };
        let Some(grab_offset) = grab_offset else {
            return;
        };

        self.interaction.drag_started();
        *self.active_drag.borrow_mut() = Some(ScrollbarActiveDrag { grab_offset });
        self.interaction.record_activity(window, cx);
    }

    fn thumb_grab_offset_at(&self, pointer_position: Point<Pixels>) -> Option<Pixels> {
        let state = self.interaction.current_state()?;
        match crate::scrollbar_pointer_down_action(self.axis, self.style, state, pointer_position) {
            ScrollbarPointerDownAction::StartDrag { grab_offset } => Some(grab_offset),
            ScrollbarPointerDownAction::Page { .. } | ScrollbarPointerDownAction::Ignore => None,
        }
    }
}

impl Drop for ScrollbarDragValue {
    fn drop(&mut self) {
        if self.active_drag.borrow_mut().take().is_some() {
            self.interaction.drag_ended();
        }
    }
}

fn update_scrollbar_drag(
    axis: Axis,
    style: ScrollbarStyle,
    interaction: &ScrollbarInteraction,
    active_drag: &Rc<RefCell<Option<ScrollbarActiveDrag>>>,
    pointer_position: Point<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(active_drag) = *active_drag.borrow() else {
        return;
    };
    if dispatch_scrollbar_drag(
        axis,
        style,
        interaction,
        pointer_position,
        active_drag.grab_offset,
    )
    .is_some()
    {
        interaction.record_activity(window, cx);
    }
}

struct ScrollbarDragPreview;

impl Render for ScrollbarDragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(0.0)).h(px(0.0))
    }
}

fn handle_scrollbar_mouse_down(
    axis: Axis,
    style: ScrollbarStyle,
    interaction: &ScrollbarInteraction,
    pending_drag: &Rc<RefCell<Option<PendingScrollbarDrag>>>,
    position: Point<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    match dispatch_scrollbar_pointer_down(axis, style, interaction, position) {
        ScrollbarPointerDownAction::StartDrag { grab_offset } => {
            *pending_drag.borrow_mut() = Some(PendingScrollbarDrag::Thumb { grab_offset });
        }
        ScrollbarPointerDownAction::Page { .. } => {
            *pending_drag.borrow_mut() = Some(PendingScrollbarDrag::Ignore);
            interaction.record_activity(window, cx);
        }
        ScrollbarPointerDownAction::Ignore => {
            *pending_drag.borrow_mut() = Some(PendingScrollbarDrag::Ignore);
        }
    }
}
