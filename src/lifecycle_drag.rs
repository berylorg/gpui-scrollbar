use gpui::{App, Pixels, Point, Window};

use crate::interaction::{dispatch_scrollbar_drag, scrollbar_drag_scroll_offset};
use crate::lifecycle::{ScrollbarActiveDrag, ScrollbarDragInstance, ScrollbarDragReceipt};
use crate::{ScrollbarGeometrySnapshot, ScrollbarState};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ScrollbarDragCompletion {
    Rejected,
    Cancelled,
    Ended,
}

impl ScrollbarDragCompletion {
    pub(crate) fn finishes_pointer(self) -> bool {
        !matches!(self, Self::Rejected)
    }
}

impl ScrollbarState {
    /// Advances only the drag addressed by `receipt` under an exact snapshot.
    pub fn update_drag(
        &self,
        receipt: &ScrollbarDragReceipt,
        pointer_position: Point<Pixels>,
    ) -> Option<(ScrollbarGeometrySnapshot, Pixels)> {
        self.receipt_is_issued_by_self(receipt)
            .then(|| self.update_drag_instance(receipt.instance, pointer_position))?
    }

    pub(crate) fn update_drag_instance(
        &self,
        expected_instance: ScrollbarDragInstance,
        pointer_position: Point<Pixels>,
    ) -> Option<(ScrollbarGeometrySnapshot, Pixels)> {
        let expected = self.exact_active(expected_instance, false)?;
        if expected
            .interaction
            .current_snapshot(expected.axis, expected.style)
            != Some(expected.expected_snapshot)
        {
            self.cancel_drag_instance(expected_instance);
            return None;
        }
        if !self.active_matches(&expected, false) {
            return None;
        }
        let next_offset = scrollbar_drag_scroll_offset(
            expected.expected_snapshot,
            pointer_position,
            expected.grab_offset,
        )?;
        if expected
            .interaction
            .current_snapshot(expected.axis, expected.style)
            != Some(expected.expected_snapshot)
        {
            self.cancel_drag_instance(expected_instance);
            return None;
        }
        if !self.active_matches(&expected, false) {
            return None;
        }
        let update = dispatch_scrollbar_drag(
            expected.expected_snapshot,
            &expected.interaction,
            next_offset,
        )?;
        let Some(current) = expected
            .interaction
            .current_snapshot(expected.axis, expected.style)
        else {
            self.cancel_drag_instance(expected_instance);
            return None;
        };
        if current.owner != expected.owner {
            self.cancel_drag_instance(expected_instance);
            return None;
        }
        let mut inner = self.inner.borrow_mut();
        if Self::matches_active(&inner, &expected, false) {
            inner
                .drag
                .as_mut()
                .expect("matched active drag")
                .expected_snapshot = current;
        }
        Some(update)
    }

    /// Completes only the drag addressed by `receipt`, exactly once.
    pub fn complete_drag(&self, receipt: &ScrollbarDragReceipt) -> bool {
        self.receipt_is_issued_by_self(receipt) && self.complete_drag_instance(receipt.instance)
    }

    pub(crate) fn complete_drag_instance(&self, expected: ScrollbarDragInstance) -> bool {
        self.settle_drag_instance(expected) == ScrollbarDragCompletion::Ended
    }

    pub(crate) fn settle_drag_instance(
        &self,
        expected_instance: ScrollbarDragInstance,
    ) -> ScrollbarDragCompletion {
        let expected = match self.exact_active(expected_instance, false) {
            Some(drag) => drag,
            None => return ScrollbarDragCompletion::Rejected,
        };
        if expected
            .interaction
            .current_snapshot(expected.axis, expected.style)
            != Some(expected.expected_snapshot)
        {
            return self.cancel_exact(&expected);
        }
        {
            let mut inner = self.inner.borrow_mut();
            if !Self::matches_active(&inner, &expected, false) {
                return ScrollbarDragCompletion::Rejected;
            }
            inner.drag.as_mut().expect("matched active drag").settling = true;
        }
        expected
            .interaction
            .end_drag_authorized(expected.expected_snapshot);
        let mut inner = self.inner.borrow_mut();
        if Self::matches_active(&inner, &expected, true) {
            inner.drag = None;
        }
        ScrollbarDragCompletion::Ended
    }

    pub(crate) fn settle_drag_from_pending(
        &self,
        expected_pending: Option<ScrollbarDragInstance>,
    ) -> ScrollbarDragCompletion {
        let active_instance = {
            let inner = self.inner.borrow();
            let Some(drag) = inner.drag.as_ref() else {
                return ScrollbarDragCompletion::Rejected;
            };
            if expected_pending.is_some_and(|pending| pending != drag.pending_instance) {
                return ScrollbarDragCompletion::Rejected;
            }
            drag.drag_instance
        };
        self.settle_drag_instance(active_instance)
    }

    pub(crate) fn owner_updated_instance(
        &self,
        expected_instance: ScrollbarDragInstance,
        window: &mut Window,
        cx: &mut App,
    ) {
        let expected = match self.exact_active(expected_instance, false) {
            Some(drag) => drag,
            None => return,
        };
        let Some(current) = expected
            .interaction
            .current_owner_update(expected.expected_snapshot, expected.style)
        else {
            return;
        };
        if current != expected.expected_snapshot || !self.active_matches(&expected, false) {
            return;
        }
        expected.interaction.owner_updated(current, window, cx);
    }

    /// Cancels only the drag addressed by `receipt` without an owner callback.
    pub fn cancel_drag(&self, receipt: &ScrollbarDragReceipt) -> bool {
        self.receipt_is_issued_by_self(receipt) && self.cancel_drag_instance(receipt.instance)
    }

    fn exact_active(
        &self,
        expected_instance: ScrollbarDragInstance,
        settling: bool,
    ) -> Option<ScrollbarActiveDrag> {
        let inner = self.inner.borrow();
        let drag = inner.drag.as_ref()?;
        (inner.owner == Some(drag.owner)
            && drag.drag_instance == expected_instance
            && drag.settling == settling)
            .then(|| drag.clone())
    }

    fn active_matches(&self, expected: &ScrollbarActiveDrag, settling: bool) -> bool {
        let inner = self.inner.borrow();
        Self::matches_active(&inner, expected, settling)
    }

    fn matches_active(
        inner: &crate::lifecycle::ScrollbarStateInner,
        expected: &ScrollbarActiveDrag,
        settling: bool,
    ) -> bool {
        inner.owner == Some(expected.owner)
            && inner.drag.as_ref().is_some_and(|drag| {
                drag.drag_instance == expected.drag_instance
                    && drag.pending_instance == expected.pending_instance
                    && drag.owner == expected.owner
                    && drag.expected_snapshot == expected.expected_snapshot
                    && drag.settling == settling
            })
    }

    fn cancel_exact(&self, expected: &ScrollbarActiveDrag) -> ScrollbarDragCompletion {
        let mut inner = self.inner.borrow_mut();
        if Self::matches_active(&inner, expected, false) {
            inner.drag = None;
            inner.visibility.invalidate();
            ScrollbarDragCompletion::Cancelled
        } else {
            ScrollbarDragCompletion::Rejected
        }
    }
}
