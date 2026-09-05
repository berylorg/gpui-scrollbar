use std::{cell::Cell, rc::Rc};

use crate::ScrollbarVisibilityKey;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FrameDriverSnapshot {
    pub driver_calls: usize,
    pub last_driver_key: Option<ScrollbarVisibilityKey>,
    pub frame_requests: usize,
    pub last_requested_key: Option<ScrollbarVisibilityKey>,
}

#[derive(Clone, Default)]
pub struct FrameDriverProbe(Rc<Cell<FrameDriverSnapshot>>);

impl FrameDriverProbe {
    pub fn snapshot(&self) -> FrameDriverSnapshot {
        self.0.get()
    }

    pub(crate) fn record_driver(&self, key: ScrollbarVisibilityKey) {
        let mut snapshot = self.snapshot();
        snapshot.driver_calls += 1;
        snapshot.last_driver_key = Some(key);
        self.0.set(snapshot);
    }

    pub(crate) fn record_request(&self, key: ScrollbarVisibilityKey) {
        let mut snapshot = self.snapshot();
        snapshot.frame_requests += 1;
        snapshot.last_requested_key = Some(key);
        self.0.set(snapshot);
    }
}
