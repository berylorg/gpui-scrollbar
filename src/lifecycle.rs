use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use gpui::Pixels;

use crate::visibility_state::ScrollbarVisibilityData;
use crate::{
    Axis, ScrollbarGeometrySnapshot, ScrollbarInteraction, ScrollbarOwnerKey, ScrollbarStyle,
};

/// Retained keyed lifecycle for one mounted scrollbar.
#[derive(Clone)]
pub struct ScrollbarState {
    pub(crate) inner: Rc<RefCell<ScrollbarStateInner>>,
}

pub(crate) struct ScrollbarStateInner {
    pub(crate) owner: Option<ScrollbarOwnerKey>,
    render_sequence: u64,
    drag_instance_sequence: u64,
    pub(crate) last_render: Option<ScrollbarRenderRecord>,
    pub(crate) pending_drag: Option<ScrollbarPendingDrag>,
    pub(crate) drag: Option<ScrollbarActiveDrag>,
    pub(crate) visibility: ScrollbarVisibilityData,
}

#[derive(Clone)]
pub(crate) struct ScrollbarPendingDrag {
    render_identity: Option<ScrollbarRenderIdentity>,
    promotable: bool,
    promotion: ScrollbarPendingPromotion,
    drag_instance: ScrollbarDragInstance,
    pub(crate) snapshot: ScrollbarGeometrySnapshot,
    pub(crate) style: ScrollbarStyle,
    pub(crate) grab_offset: Pixels,
    interaction: ScrollbarInteraction,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ScrollbarPendingPromotion {
    Ready,
    Promoting,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct ScrollbarRenderIdentity(u64);

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct ScrollbarDragInstance(u64);

/// Opaque receipt for one successfully started custom-renderer drag.
///
/// The crate issues a receipt only after the exact owner, geometry, and
/// interaction snapshot have started a drag. It is privately and
/// non-retentively bound to that retained-state allocation. It has no
/// constructor, clone, identity accessor, or mutable authority; callers retain
/// it solely to address later movement, completion, or cancellation for that
/// same drag.
pub struct ScrollbarDragReceipt {
    pub(crate) origin: Weak<RefCell<ScrollbarStateInner>>,
    pub(crate) instance: ScrollbarDragInstance,
}

impl ScrollbarDragReceipt {
    /// Returns whether the issuing retained state has already been dropped.
    ///
    /// This reports only receipt usability; it exposes neither the issuing
    /// state's identity nor a retained or mutable reference to it.
    #[must_use]
    pub fn is_orphaned(&self) -> bool {
        self.origin.upgrade().is_none()
    }
}

#[derive(Clone)]
pub(crate) struct ScrollbarRenderRecord {
    identity: ScrollbarRenderIdentity,
    snapshot: ScrollbarGeometrySnapshot,
    style: ScrollbarStyle,
    interaction: ScrollbarInteraction,
}

#[derive(Clone)]
pub(crate) struct ScrollbarActiveDrag {
    pub(crate) pending_instance: ScrollbarDragInstance,
    pub(crate) drag_instance: ScrollbarDragInstance,
    pub(crate) owner: ScrollbarOwnerKey,
    pub(crate) axis: Axis,
    pub(crate) style: ScrollbarStyle,
    pub(crate) grab_offset: Pixels,
    pub(crate) expected_snapshot: ScrollbarGeometrySnapshot,
    pub(crate) interaction: ScrollbarInteraction,
    pub(crate) settling: bool,
}

impl ScrollbarState {
    /// Creates retained state mounted under an exact owner key.
    #[must_use]
    pub fn new(owner: ScrollbarOwnerKey) -> Self {
        Self {
            inner: Rc::new(RefCell::new(ScrollbarStateInner {
                owner: Some(owner),
                render_sequence: 0,
                drag_instance_sequence: 0,
                last_render: None,
                pending_drag: None,
                drag: None,
                visibility: ScrollbarVisibilityData::new(),
            })),
        }
    }

    pub(crate) fn pending_drag_instance(
        &self,
        render_identity: ScrollbarRenderIdentity,
    ) -> ScrollbarDragInstance {
        let mut inner = self.inner.borrow_mut();
        if let Some(pending) = inner.pending_drag.as_ref()
            && pending.render_identity == Some(render_identity)
        {
            return pending.drag_instance;
        }
        inner.drag_instance_sequence = inner
            .drag_instance_sequence
            .checked_add(1)
            .expect("scrollbar drag instance sequence exhausted");
        ScrollbarDragInstance(inner.drag_instance_sequence)
    }

    pub(crate) fn next_drag_instance(&self) -> ScrollbarDragInstance {
        let mut inner = self.inner.borrow_mut();
        inner.drag_instance_sequence = inner
            .drag_instance_sequence
            .checked_add(1)
            .expect("scrollbar drag instance sequence exhausted");
        ScrollbarDragInstance(inner.drag_instance_sequence)
    }

    pub(crate) fn active_drag_instance(&self) -> Option<ScrollbarDragInstance> {
        self.inner
            .borrow()
            .drag
            .as_ref()
            .map(|drag| drag.drag_instance)
    }

    pub(crate) fn claim_render_identity(
        &self,
        snapshot: ScrollbarGeometrySnapshot,
        style: ScrollbarStyle,
        interaction: &ScrollbarInteraction,
    ) -> Option<ScrollbarRenderIdentity> {
        let mut inner = self.inner.borrow_mut();
        if inner.owner != Some(snapshot.owner) {
            return None;
        }
        if let Some(last) = inner.last_render.as_ref()
            && last.snapshot == snapshot
            && last.style == style
            && last.interaction.is_same_instance(interaction)
        {
            return Some(last.identity);
        }
        inner.render_sequence = inner
            .render_sequence
            .checked_add(1)
            .expect("scrollbar render sequence exhausted");
        let identity = ScrollbarRenderIdentity(inner.render_sequence);
        inner.last_render = Some(ScrollbarRenderRecord {
            identity,
            snapshot,
            style,
            interaction: interaction.clone(),
        });
        if let Some(pending) = inner.pending_drag.as_mut() {
            // Retain this exact press for the latest-render release handler.
            pending.render_identity = Some(identity);
            pending.promotable = false;
        }
        Some(identity)
    }

    pub(crate) fn retire_render_constructor(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.render_sequence = inner
            .render_sequence
            .checked_add(1)
            .expect("scrollbar render sequence exhausted");
        inner.last_render = None;
        inner.pending_drag = None;
    }

    /// Starts one keyed drag from the exact hit-tested snapshot.
    pub(crate) fn begin_pending_drag(
        &self,
        render_identity: ScrollbarRenderIdentity,
        drag_instance: ScrollbarDragInstance,
        style: ScrollbarStyle,
        snapshot: ScrollbarGeometrySnapshot,
        grab_offset: Pixels,
        interaction: &ScrollbarInteraction,
    ) -> bool {
        let mut inner = self.inner.borrow_mut();
        if inner.owner != Some(snapshot.owner)
            || inner.pending_drag.is_some()
            || inner.drag.is_some()
        {
            return false;
        }
        inner.pending_drag = Some(ScrollbarPendingDrag {
            render_identity: Some(render_identity),
            promotable: true,
            promotion: ScrollbarPendingPromotion::Ready,
            drag_instance,
            snapshot,
            style,
            grab_offset,
            interaction: interaction.clone(),
        });
        true
    }

    pub(crate) fn cancel_pending_drag(&self, expected: ScrollbarDragInstance) -> bool {
        let mut inner = self.inner.borrow_mut();
        if !inner
            .pending_drag
            .as_ref()
            .is_some_and(|pending| pending.drag_instance == expected)
        {
            return false;
        }
        inner.pending_drag = None;
        true
    }

    pub(crate) fn cancel_drag_instance(&self, expected: ScrollbarDragInstance) -> bool {
        let mut inner = self.inner.borrow_mut();
        if inner
            .drag
            .as_ref()
            .is_some_and(|active| active.drag_instance == expected)
        {
            inner.drag = None;
            inner.visibility.invalidate();
            return true;
        }
        false
    }

    /// Promotes the exact pending press into one keyed drag.
    pub(crate) fn start_pending_drag(
        &self,
        expected_render: ScrollbarRenderIdentity,
        expected_pending: ScrollbarDragInstance,
        active_instance: ScrollbarDragInstance,
        interaction: &ScrollbarInteraction,
    ) -> bool {
        self.promote_pending_drag(
            Some(expected_render),
            expected_pending,
            active_instance,
            interaction,
        ) == Some(true)
    }

    fn promote_pending_drag(
        &self,
        expected_render: Option<ScrollbarRenderIdentity>,
        expected_pending: ScrollbarDragInstance,
        active_instance: ScrollbarDragInstance,
        interaction: &ScrollbarInteraction,
    ) -> Option<bool> {
        let pending = {
            let mut inner = self.inner.borrow_mut();
            let owner = inner.owner;
            let no_drag = inner.drag.is_none();
            let Some(pending) = inner.pending_drag.as_mut() else {
                return None;
            };
            if pending.render_identity != expected_render
                || pending.drag_instance != expected_pending
                || !pending.promotable
                || pending.promotion != ScrollbarPendingPromotion::Ready
                || !pending.interaction.is_same_instance(interaction)
                || owner != Some(pending.snapshot.owner)
                || !no_drag
            {
                return None;
            }
            pending.promotion = ScrollbarPendingPromotion::Promoting;
            pending.clone()
        };
        if interaction.current_snapshot(pending.snapshot.axis, pending.style)
            != Some(pending.snapshot)
        {
            self.cancel_promoting_pending(expected_render, &pending);
            return None;
        }
        {
            let mut inner = self.inner.borrow_mut();
            if inner.owner != Some(pending.snapshot.owner)
                || inner.drag.is_some()
                || !Self::matches_promoting_pending(&inner, expected_render, &pending, interaction)
            {
                return None;
            }
            inner.pending_drag = None;
            inner.drag = Some(ScrollbarActiveDrag {
                pending_instance: pending.drag_instance,
                drag_instance: active_instance,
                owner: pending.snapshot.owner,
                axis: pending.snapshot.axis,
                style: pending.style,
                grab_offset: pending.grab_offset,
                expected_snapshot: pending.snapshot,
                interaction: interaction.clone(),
                settling: false,
            });
        }
        if !interaction.start_drag(pending.snapshot, pending.style) {
            self.cancel_drag_instance(active_instance);
            return None;
        }
        let remains_current = self.inner.borrow().owner == Some(pending.snapshot.owner)
            && self.inner.borrow().drag.as_ref().is_some_and(|drag| {
                drag.pending_instance == pending.drag_instance
                    && drag.drag_instance == active_instance
            });
        Some(remains_current)
    }

    fn cancel_promoting_pending(
        &self,
        expected_render: Option<ScrollbarRenderIdentity>,
        expected: &ScrollbarPendingDrag,
    ) {
        let mut inner = self.inner.borrow_mut();
        if Self::matches_promoting_pending(&inner, expected_render, expected, &expected.interaction)
        {
            inner.pending_drag = None;
        }
    }

    fn matches_promoting_pending(
        inner: &ScrollbarStateInner,
        expected_render: Option<ScrollbarRenderIdentity>,
        expected: &ScrollbarPendingDrag,
        interaction: &ScrollbarInteraction,
    ) -> bool {
        inner.owner == Some(expected.snapshot.owner)
            && inner.pending_drag.as_ref().is_some_and(|pending| {
                pending.render_identity == expected_render
                    && pending.promotable
                    && pending.promotion == ScrollbarPendingPromotion::Promoting
                    && pending.drag_instance == expected.drag_instance
                    && pending.snapshot == expected.snapshot
                    && pending.style == expected.style
                    && pending.grab_offset == expected.grab_offset
                    && pending.interaction.is_same_instance(interaction)
            })
    }

    /// Starts one exact custom-renderer drag and returns its opaque receipt.
    ///
    /// Once the start callback runs, this returns that receipt even if
    /// reentrancy settled it. Pre-start rejection returns `None`.
    pub fn start_drag(
        &self,
        interaction: &ScrollbarInteraction,
        style: ScrollbarStyle,
        snapshot: ScrollbarGeometrySnapshot,
        grab_offset: Pixels,
    ) -> Option<ScrollbarDragReceipt> {
        let drag_instance = self.next_drag_instance();
        {
            let mut inner = self.inner.borrow_mut();
            if inner.owner != Some(snapshot.owner)
                || inner.pending_drag.is_some()
                || inner.drag.is_some()
            {
                return None;
            }
            inner.pending_drag = Some(ScrollbarPendingDrag {
                render_identity: None,
                promotable: true,
                promotion: ScrollbarPendingPromotion::Ready,
                drag_instance,
                snapshot,
                style,
                grab_offset,
                interaction: interaction.clone(),
            });
        }
        self.promote_pending_drag(None, drag_instance, drag_instance, interaction)
            .map(|_| ScrollbarDragReceipt {
                origin: Rc::downgrade(&self.inner),
                instance: drag_instance,
            })
    }

    pub(crate) fn receipt_is_issued_by_self(&self, receipt: &ScrollbarDragReceipt) -> bool {
        receipt
            .origin
            .upgrade()
            .is_some_and(|origin| Rc::ptr_eq(&origin, &self.inner))
    }

    pub(crate) fn has_current_owner(&self, expected: ScrollbarOwnerKey) -> bool {
        self.inner.borrow().owner == Some(expected)
    }
}
