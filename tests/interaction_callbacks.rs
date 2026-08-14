mod support;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Instant,
};

use gpui::px;
use gpui_scrollbar::{
    Axis, ScrollbarInteraction, ScrollbarState, ScrollbarStyle, scrollbar_geometry_snapshot,
};
use support::{key, vertical_state};

#[test]
fn started_callback_geometry_change_returns_receipt_for_exact_cancellation() {
    let owner = key(6, 1);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(owner, px(0.0))));
    let state = ScrollbarState::new(owner);
    let started = Rc::new(Cell::new(0));
    let interaction = ScrollbarInteraction::new(
        {
            let current = current.clone();
            move || Some(current.get())
        },
        |_, _| {},
        |_, _, _| {},
        {
            let current = current.clone();
            let state = state.clone();
            let started = started.clone();
            move |_| {
                started.set(started.get() + 1);
                current.set(vertical_state(owner, px(1.0)));
                assert!(state.record_activity_at(owner, Instant::now()).is_some());
            }
        },
        |_| {},
        |_, _, _| {},
    );
    let snapshot =
        scrollbar_geometry_snapshot(Axis::Vertical, style.geometry, current.get()).unwrap();

    let receipt = state
        .start_drag(&interaction, style, snapshot, px(10.0))
        .expect("started callback cannot strand its caller without a receipt");
    assert_eq!(started.get(), 1);
    assert!(state.cancel_drag(&receipt));
    assert!(!state.cancel_drag(&receipt));
    assert!(!state.complete_drag(&receipt));
}

#[test]
fn completion_snapshot_provider_can_cancel_exact_receipt_reentrantly() {
    let owner = key(7, 1);
    let style = ScrollbarStyle::default();
    let current = Rc::new(Cell::new(vertical_state(owner, px(0.0))));
    let state = ScrollbarState::new(owner);
    let receipt_slot = Rc::new(RefCell::new(None));
    let cancel_on_provider = Rc::new(Cell::new(false));
    let interaction = ScrollbarInteraction::new(
        {
            let current = current.clone();
            let state = state.clone();
            let receipt_slot = receipt_slot.clone();
            let cancel_on_provider = cancel_on_provider.clone();
            move || {
                if cancel_on_provider.replace(false) {
                    let receipt = receipt_slot.borrow();
                    assert!(state.cancel_drag(receipt.as_deref().expect("published receipt")));
                }
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
    let receipt = Rc::new(
        state
            .start_drag(&interaction, style, snapshot, px(10.0))
            .expect("drag starts"),
    );
    receipt_slot.replace(Some(receipt.clone()));
    cancel_on_provider.set(true);

    assert!(!state.complete_drag(&receipt));
    assert!(!state.complete_drag(&receipt));
}
