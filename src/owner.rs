use gpui::{App, Window};

use crate::{ScrollbarOwnerKey, ScrollbarState};

impl ScrollbarState {
    /// Returns the currently mounted owner key, if any.
    #[must_use]
    pub fn current_owner(&self) -> Option<ScrollbarOwnerKey> {
        self.inner.borrow().owner
    }

    /// Mounts this retained state after an exact unmount.
    ///
    /// Returns false when another owner is still mounted.
    pub fn mount(&self, owner: ScrollbarOwnerKey) -> bool {
        let mut inner = self.inner.borrow_mut();
        if inner.owner.is_some() {
            return false;
        }
        inner.owner = Some(owner);
        inner.last_render = None;
        inner.pending_drag = None;
        inner.visibility.invalidate();
        true
    }

    /// Replaces the exact current owner and cancels all prior owned work.
    pub fn replace_owner(
        &self,
        expected: ScrollbarOwnerKey,
        replacement: ScrollbarOwnerKey,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        self.replace_or_unmount(expected, Some(replacement), window, cx)
    }

    /// Unmounts the owning viewport and cancels all scrollbar-owned work.
    pub fn unmount_viewport(
        &self,
        expected: ScrollbarOwnerKey,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        self.replace_or_unmount(expected, None, window, cx)
    }

    /// Unmounts the scrollbar chrome and cancels all scrollbar-owned work.
    pub fn unmount_scrollbar(
        &self,
        expected: ScrollbarOwnerKey,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        self.replace_or_unmount(expected, None, window, cx)
    }

    /// Tears down the exact window-owned scrollbar lifecycle.
    pub fn teardown_window(
        &self,
        expected: ScrollbarOwnerKey,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        self.replace_or_unmount(expected, None, window, cx)
    }

    fn replace_or_unmount(
        &self,
        expected: ScrollbarOwnerKey,
        replacement: Option<ScrollbarOwnerKey>,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        let active_drag = {
            let mut inner = self.inner.borrow_mut();
            if inner.owner != Some(expected) {
                return false;
            }
            let active_drag = inner.drag.take();
            inner.pending_drag = None;
            active_drag
        };
        {
            let mut inner = self.inner.borrow_mut();
            if inner.owner != Some(expected) {
                return false;
            }
            inner.owner = replacement;
            inner.last_render = None;
            inner.visibility.invalidate();
        }
        if active_drag.is_some() {
            cx.stop_active_drag(window);
        }
        true
    }
}
