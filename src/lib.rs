#![forbid(unsafe_code)]
//! App-neutral scrollbar primitives for `gpui` applications.
//!
//! The crate owns reusable scrollbar concepts and provides shared geometry,
//! rendering, keyed managed visibility, and pointer direct manipulation.
//! Callers retain scroll state, focus, keyboard and wheel routing, and edge
//! policy. Callback interactions are bound to an exact owner and mount key;
//! every callback receives the same geometry snapshot that authorized it.
//! Recreated scroll-handle adapters preserve interaction continuity for clones
//! of one actual handle and reject separately constructed equal-state handles.
//! Custom interactive renderers settle their own drag only through opaque
//! crate-issued receipts.
//!
//! ```
//! use gpui::{Bounds, point, px, size};
//! use gpui_scrollbar::{
//!     Axis, ScrollbarInteraction, ScrollbarMountGeneration, ScrollbarOwnerId,
//!     ScrollbarOwnerKey, ScrollbarScrollState, ScrollbarState, ScrollbarStyle,
//! };
//! use std::{cell::Cell, rc::Rc};
//!
//! let owner = ScrollbarOwnerKey::new(
//!     ScrollbarOwnerId::new(7),
//!     ScrollbarMountGeneration::new(3),
//! );
//! let retained = ScrollbarState::new(owner);
//! let scroll = Rc::new(Cell::new(ScrollbarScrollState {
//!         owner,
//!         viewport_bounds: Bounds::new(point(px(0.0), px(0.0)), size(px(240.0), px(240.0))),
//!         content_size: size(px(240.0), px(480.0)),
//!         scroll_offset: point(px(0.0), px(120.0)),
//!         page_distance: size(px(240.0), px(180.0)),
//! }));
//! let interaction = ScrollbarInteraction::new(
//!     {
//!         let scroll = scroll.clone();
//!         move || Some(scroll.get())
//!     },
//!     {
//!         let scroll = scroll.clone();
//!         move |snapshot, offset| {
//!             assert_eq!(snapshot.owner, owner);
//!             let mut current = scroll.get();
//!             current.scroll_offset.y = offset;
//!             scroll.set(current);
//!         }
//!     },
//!     |snapshot, _, distance| {
//!         assert_eq!(snapshot.page_distance, distance);
//!     },
//!     |snapshot| assert_eq!(snapshot.owner, owner),
//!     |snapshot| assert_eq!(snapshot.owner, owner),
//!     |snapshot, _, _| assert_eq!(snapshot.owner, owner),
//! );
//! let snapshot = interaction
//!     .current_snapshot(Axis::Vertical, ScrollbarStyle::default())
//!     .expect("overflowing keyed geometry");
//! assert_eq!(snapshot.content_length, px(480.0));
//! assert_eq!(retained.current_owner(), Some(owner));
//! ```

mod geometry;
mod identity;
mod interaction;
mod lifecycle;
mod lifecycle_drag;
mod owner;
mod render;
mod style;
#[cfg(feature = "test-support")]
pub mod test_support;
mod visibility;
mod visibility_state;

pub use geometry::{
    ScrollbarAxisBounds, ScrollbarAxisHit, ScrollbarGeometrySnapshot, ScrollbarGeometryStyle,
    ScrollbarMetrics, ScrollbarScrollState, ScrollbarTrackGeometry, classify_scrollbar_axis_hit,
    scroll_offset_from_thumb_drag, scrollbar_geometry_snapshot, scrollbar_metrics,
    scrollbar_thumb_grab_offset, scrollbar_track_geometry,
};
pub use identity::{
    ScrollbarMountGeneration, ScrollbarOwnerId, ScrollbarOwnerKey, ScrollbarVisibilityKey,
    ScrollbarVisibilitySequence,
};
pub use interaction::{
    ScrollbarInteraction, ScrollbarOwnerUpdateCallback, ScrollbarPointerDownAction,
    dispatch_scrollbar_pointer_down, scrollbar_drag_scroll_offset, scrollbar_pointer_down_action,
};
pub use lifecycle::{ScrollbarDragReceipt, ScrollbarState};
pub use render::{render_scroll_handle_scrollbar, render_scrollbar, render_scrollbar_thumb};
pub use style::ScrollbarStyle;
pub use visibility::{
    ManagedScrollbarVisibility, ScrollbarFadeConfig, ScrollbarVisibilityPolicy,
    ScrollbarVisibilityUpdateCallback,
};

/// The visual and movement orientation of a scrollbar.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Axis {
    /// A scrollbar that moves along the vertical axis.
    Vertical,
    /// A scrollbar that moves along the horizontal axis.
    Horizontal,
}

impl Axis {
    /// Returns true for [`Axis::Vertical`].
    #[must_use]
    pub const fn is_vertical(self) -> bool {
        matches!(self, Self::Vertical)
    }

    /// Returns true for [`Axis::Horizontal`].
    #[must_use]
    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::Horizontal)
    }
}

/// Direction for one page of caller-owned scrolling.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ScrollDirection {
    /// Scroll toward the start of the scrollable content.
    Backward,
    /// Scroll toward the end of the scrollable content.
    Forward,
}

/// A click in the scrollbar lane outside the current thumb.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum LaneClick {
    /// The click is before the thumb on the scrollbar axis.
    BeforeThumb,
    /// The click is after the thumb on the scrollbar axis.
    AfterThumb,
}

impl LaneClick {
    /// Maps a lane click to the page-scroll direction that callers should apply.
    #[must_use]
    pub const fn page_direction(self) -> ScrollDirection {
        match self {
            Self::BeforeThumb => ScrollDirection::Backward,
            Self::AfterThumb => ScrollDirection::Forward,
        }
    }
}
