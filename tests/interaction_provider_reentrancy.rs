mod support;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui::{point, px};
use gpui_scrollbar::{
    Axis, ScrollbarInteraction, ScrollbarState, ScrollbarStyle, dispatch_scrollbar_pointer_down,
    scrollbar_geometry_snapshot,
};
use support::{key, vertical_state};

#[test]
fn completion_provider_cancel_a_start_b_preserves_b() {
    let owner = key(70, 1);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(owner, px(0.0))));
    let state = ScrollbarState::new(owner);
    let first_receipt = Rc::new(RefCell::new(None));
    let second_receipt = Rc::new(RefCell::new(None));
    let interaction_slot = Rc::new(RefCell::new(None));
    let reenter = Rc::new(Cell::new(false));
    let ended = Rc::new(Cell::new(0));
    let interaction = ScrollbarInteraction::new(
        {
            let current = current.clone();
            let state = state.clone();
            let first_receipt = first_receipt.clone();
            let second_receipt = second_receipt.clone();
            let interaction_slot = interaction_slot.clone();
            let reenter = reenter.clone();
            move || {
                if reenter.replace(false) {
                    assert!(state.cancel_drag(first_receipt.borrow().as_deref().expect("A")));
                    let snapshot =
                        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get())
                            .expect("B geometry");
                    let interaction = interaction_slot.borrow();
                    let receipt = state
                        .start_drag(
                            interaction.as_ref().expect("interaction"),
                            style,
                            snapshot,
                            px(8.0),
                        )
                        .expect("B starts");
                    second_receipt.replace(Some(Rc::new(receipt)));
                }
                Some(current.get())
            }
        },
        |_, _| {},
        |_, _, _| {},
        |_| {},
        {
            let ended = ended.clone();
            move |_| ended.set(ended.get() + 1)
        },
        |_, _, _| {},
    );
    interaction_slot.replace(Some(interaction.clone()));
    let snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();
    let first = Rc::new(
        state
            .start_drag(&interaction, style, snapshot, px(8.0))
            .expect("A starts"),
    );
    first_receipt.replace(Some(first.clone()));
    reenter.set(true);

    assert!(!state.complete_drag(&first));
    let second = second_receipt.borrow();
    let second = second.as_deref().expect("B receipt survives");
    assert!(
        state
            .update_drag(second, point(px(236.0), px(130.0)))
            .is_some()
    );
    assert!(state.complete_drag(second));
    assert_eq!(ended.get(), 1);
}

#[test]
fn update_provider_cancel_a_start_b_suppresses_a_scroll_callback() {
    let owner = key(71, 1);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(owner, px(0.0))));
    let state = ScrollbarState::new(owner);
    let first_receipt = Rc::new(RefCell::new(None));
    let second_receipt = Rc::new(RefCell::new(None));
    let interaction_slot = Rc::new(RefCell::new(None));
    let reenter = Rc::new(Cell::new(false));
    let writes = Rc::new(Cell::new(0));
    let interaction = ScrollbarInteraction::new(
        {
            let current = current.clone();
            let state = state.clone();
            let first_receipt = first_receipt.clone();
            let second_receipt = second_receipt.clone();
            let interaction_slot = interaction_slot.clone();
            let reenter = reenter.clone();
            move || {
                if reenter.replace(false) {
                    assert!(state.cancel_drag(first_receipt.borrow().as_deref().expect("A")));
                    let snapshot =
                        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get())
                            .expect("B geometry");
                    let interaction = interaction_slot.borrow();
                    let receipt = state
                        .start_drag(
                            interaction.as_ref().expect("interaction"),
                            style,
                            snapshot,
                            px(8.0),
                        )
                        .expect("B starts");
                    second_receipt.replace(Some(Rc::new(receipt)));
                }
                Some(current.get())
            }
        },
        {
            let writes = writes.clone();
            move |_, _| writes.set(writes.get() + 1)
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
        state
            .start_drag(&interaction, style, snapshot, px(8.0))
            .expect("A starts"),
    );
    first_receipt.replace(Some(first.clone()));
    reenter.set(true);

    assert!(
        state
            .update_drag(&first, point(px(236.0), px(130.0)))
            .is_none()
    );
    assert_eq!(writes.get(), 0);
    let second = second_receipt.borrow();
    let second = second.as_deref().expect("B receipt survives");
    assert!(
        state
            .update_drag(second, point(px(236.0), px(130.0)))
            .is_some()
    );
    assert_eq!(writes.get(), 1);
    assert!(state.complete_drag(second));
}

#[test]
fn end_callback_cancel_a_start_b_leaves_b_settleable() {
    let owner = key(72, 1);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(owner, px(0.0))));
    let state = ScrollbarState::new(owner);
    let first_receipt = Rc::new(RefCell::new(None));
    let second_receipt = Rc::new(RefCell::new(None));
    let interaction_slot = Rc::new(RefCell::new(None));
    let reenter = Rc::new(Cell::new(true));
    let interaction = ScrollbarInteraction::new(
        {
            let current = current.clone();
            move || Some(current.get())
        },
        |_, _| {},
        |_, _, _| {},
        |_| {},
        {
            let state = state.clone();
            let first_receipt = first_receipt.clone();
            let second_receipt = second_receipt.clone();
            let interaction_slot = interaction_slot.clone();
            let current = current.clone();
            let reenter = reenter.clone();
            move |_| {
                if !reenter.replace(false) {
                    return;
                }
                assert!(state.cancel_drag(first_receipt.borrow().as_deref().expect("A")));
                let snapshot =
                    scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get())
                        .expect("B geometry");
                let interaction = interaction_slot.borrow();
                let receipt = state
                    .start_drag(
                        interaction.as_ref().expect("interaction"),
                        style,
                        snapshot,
                        px(8.0),
                    )
                    .expect("B starts after exact A cancellation");
                second_receipt.replace(Some(Rc::new(receipt)));
            }
        },
        |_, _, _| {},
    );
    interaction_slot.replace(Some(interaction.clone()));
    let snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();
    let first = Rc::new(
        state
            .start_drag(&interaction, style, snapshot, px(8.0))
            .expect("A starts"),
    );
    first_receipt.replace(Some(first.clone()));

    assert!(state.complete_drag(&first));
    let second = second_receipt.borrow();
    let second = second.as_deref().expect("B receipt survives A cleanup");
    assert!(
        state
            .update_drag(second, point(px(236.0), px(130.0)))
            .is_some()
    );
    assert!(state.complete_drag(second));
}

#[test]
fn lane_final_provider_change_suppresses_page_callback() {
    let owner = key(73, 1);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(owner, px(120.0))));
    let provider_calls = Rc::new(Cell::new(0));
    let pages = Rc::new(Cell::new(0));
    let interaction = ScrollbarInteraction::new(
        {
            let current = current.clone();
            let provider_calls = provider_calls.clone();
            move || {
                if provider_calls.get() == 1 {
                    current.set(vertical_state(owner, px(121.0)));
                }
                provider_calls.set(provider_calls.get() + 1);
                Some(current.get())
            }
        },
        |_, _| {},
        {
            let pages = pages.clone();
            move |_, _, _| pages.set(pages.get() + 1)
        },
        |_| {},
        |_| {},
        |_, _, _| {},
    );
    let state = ScrollbarState::new(owner);
    let snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();

    assert!(matches!(
        dispatch_scrollbar_pointer_down(
            &state,
            style,
            &interaction,
            snapshot,
            point(px(236.0), snapshot.thumb_bounds.end + px(1.0)),
        ),
        gpui_scrollbar::ScrollbarPointerDownAction::Ignore
    ));
    assert_eq!(pages.get(), 0);
}

#[test]
fn pending_provider_cannot_start_recurrent_drag_while_a_is_promoting() {
    let owner = key(74, 1);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(owner, px(0.0))));
    let state = ScrollbarState::new(owner);
    let recurrent_start_rejected = Rc::new(Cell::new(false));
    let recurrent = ScrollbarInteraction::new(
        {
            let current = current.clone();
            move || Some(current.get())
        },
        |_, _| {},
        |_, _, _| {},
        |_| {},
        |_| {},
        |_, _, _| {},
    );
    let interaction = ScrollbarInteraction::new(
        {
            let current = current.clone();
            let state = state.clone();
            let recurrent = recurrent.clone();
            let recurrent_start_rejected = recurrent_start_rejected.clone();
            move || {
                let snapshot =
                    scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get())
                        .expect("recurring geometry");
                recurrent_start_rejected.set(
                    state
                        .start_drag(&recurrent, style, snapshot, px(8.0))
                        .is_none(),
                );
                Some(current.get())
            }
        },
        |_, _| {},
        |_, _, _| {},
        |_| {},
        |_| {},
        |_, _, _| {},
    );
    let snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();

    let receipt = state
        .start_drag(&interaction, style, snapshot, px(8.0))
        .expect("A promotes after retained pending provider check");
    assert!(recurrent_start_rejected.get());
    assert!(
        state
            .update_drag(&receipt, point(px(236.0), px(130.0)))
            .is_some()
    );
    assert!(state.complete_drag(&receipt));
}
