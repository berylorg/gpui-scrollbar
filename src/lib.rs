#![forbid(unsafe_code)]
//! App-neutral scrollbar primitives for `gpui` applications.
//!
//! The crate owns reusable scrollbar concepts and later provides the shared
//! geometry, rendering, and pointer direct-manipulation behavior. Callers keep
//! ownership of scroll state, focus routing, keyboard input, wheel routing,
//! visibility policy, and application-specific edge rules.
//!
//! ```
//! use gpui::{Bounds, point, px, size};
//! use gpui_scrollbar::{
//!     Axis, LaneClick, ScrollDirection, ScrollbarGeometryStyle, ScrollbarPointerDownAction,
//!     ScrollbarScrollState, ScrollbarStyle, scrollbar_metrics, scrollbar_pointer_down_action,
//! };
//!
//! assert!(Axis::Vertical.is_vertical());
//! assert_eq!(LaneClick::AfterThumb.page_direction(), ScrollDirection::Forward);
//!
//! let metrics = scrollbar_metrics(
//!     ScrollbarGeometryStyle::default(),
//!     px(240.0),
//!     px(240.0),
//!     px(120.0),
//! )
//! .expect("overflowing content has visible scrollbar metrics");
//! assert!(metrics.thumb_offset > px(0.0));
//!
//! let action = scrollbar_pointer_down_action(
//!     Axis::Vertical,
//!     ScrollbarStyle::default(),
//!     ScrollbarScrollState {
//!         viewport_bounds: Bounds::new(point(px(0.0), px(0.0)), size(px(240.0), px(240.0))),
//!         max_offset: size(px(0.0), px(240.0)),
//!         scroll_offset: point(px(0.0), px(120.0)),
//!     },
//!     point(px(236.0), px(230.0)),
//! );
//! assert!(matches!(action, ScrollbarPointerDownAction::Page { .. }));
//! ```

mod geometry;
mod interaction;
mod render;
mod style;

pub use geometry::{
    ScrollbarAxisHit, ScrollbarGeometryStyle, ScrollbarMetrics, ScrollbarTrackGeometry,
    classify_scrollbar_axis_hit, scroll_offset_from_thumb_drag, scrollbar_metrics,
    scrollbar_thumb_grab_offset, scrollbar_track_geometry,
};
pub use interaction::{
    ScrollbarActivityCallback, ScrollbarInteraction, ScrollbarPointerDownAction,
    ScrollbarScrollState, dispatch_scrollbar_drag, dispatch_scrollbar_pointer_down,
    scrollbar_drag_scroll_offset, scrollbar_pointer_down_action,
};
pub use render::{render_scroll_handle_scrollbar, render_scrollbar, render_scrollbar_thumb};
pub use style::ScrollbarStyle;

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
