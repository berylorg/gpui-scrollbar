use gpui::{Pixels, px};

/// App-neutral geometry constants used by scrollbar math.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarGeometryStyle {
    /// Space reserved at each end of the scrollbar track.
    pub track_inset: Pixels,
    /// Smallest thumb size allowed before clamping to the track length.
    pub min_thumb_length: Pixels,
}

impl Default for ScrollbarGeometryStyle {
    fn default() -> Self {
        Self {
            track_inset: px(6.0),
            min_thumb_length: px(24.0),
        }
    }
}

impl ScrollbarGeometryStyle {
    pub(crate) fn normalized(self) -> Self {
        Self {
            track_inset: self.track_inset.max(px(0.0)),
            min_thumb_length: self.min_thumb_length.max(px(0.0)),
        }
    }
}

/// Derived thumb metrics for a visible scrollbar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarMetrics {
    /// Offset of the thumb from the start of the track.
    pub thumb_offset: Pixels,
    /// Length of the thumb along the scrollbar axis.
    pub thumb_length: Pixels,
}

/// Location of a pointer hit along the scrollbar axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollbarAxisHit {
    /// The hit is in the lane before the thumb.
    LaneBeforeThumb,
    /// The hit is on the thumb.
    Thumb,
    /// The hit is in the lane after the thumb.
    LaneAfterThumb,
}

/// Track and thumb bounds along the scrollbar axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarTrackGeometry {
    /// Start of the track relative to the viewport.
    pub track_start: Pixels,
    /// Length of the track along the scrollbar axis.
    pub track_length: Pixels,
    /// Start of the thumb relative to the viewport.
    pub thumb_start: Pixels,
    /// End of the thumb relative to the viewport.
    pub thumb_end: Pixels,
}

/// Computes visible scrollbar thumb metrics for a viewport and overflow amount.
///
/// Returns `None` when the viewport is empty, when there is no overflow, or
/// when the configured track has no positive length.
#[must_use]
pub fn scrollbar_metrics(
    style: ScrollbarGeometryStyle,
    viewport_length: Pixels,
    overflow_length: Pixels,
    scroll_offset: Pixels,
) -> Option<ScrollbarMetrics> {
    let style = style.normalized();
    if viewport_length <= px(0.0) || overflow_length <= px(0.0) {
        return None;
    }

    let track_length = (viewport_length - style.track_inset * 2.0).max(px(0.0));
    if track_length <= px(0.0) {
        return None;
    }

    let content_length = viewport_length + overflow_length;
    let thumb_length = (track_length * (viewport_length / content_length))
        .max(style.min_thumb_length)
        .min(track_length);
    let thumb_travel = (track_length - thumb_length).max(px(0.0));
    let progress =
        (scroll_offset.clamp(px(0.0), overflow_length) / overflow_length).clamp(0.0, 1.0);

    Some(ScrollbarMetrics {
        thumb_offset: thumb_travel * progress,
        thumb_length,
    })
}

/// Computes track and thumb bounds for visible scrollbar metrics.
#[must_use]
pub fn scrollbar_track_geometry(
    style: ScrollbarGeometryStyle,
    viewport_length: Pixels,
    metrics: ScrollbarMetrics,
) -> Option<ScrollbarTrackGeometry> {
    let style = style.normalized();
    let track_length = (viewport_length - style.track_inset * 2.0).max(px(0.0));
    (track_length > px(0.0)).then_some(ScrollbarTrackGeometry {
        track_start: style.track_inset,
        track_length,
        thumb_start: style.track_inset + metrics.thumb_offset,
        thumb_end: style.track_inset + metrics.thumb_offset + metrics.thumb_length,
    })
}

/// Classifies a pointer position along the scrollbar axis.
#[must_use]
pub fn classify_scrollbar_axis_hit(
    style: ScrollbarGeometryStyle,
    viewport_length: Pixels,
    metrics: ScrollbarMetrics,
    axis_position: Pixels,
) -> Option<ScrollbarAxisHit> {
    let geometry = scrollbar_track_geometry(style, viewport_length, metrics)?;
    if axis_position < geometry.thumb_start {
        Some(ScrollbarAxisHit::LaneBeforeThumb)
    } else if axis_position <= geometry.thumb_end {
        Some(ScrollbarAxisHit::Thumb)
    } else {
        Some(ScrollbarAxisHit::LaneAfterThumb)
    }
}

/// Returns the pointer's offset from the thumb start, clamped to the thumb.
#[must_use]
pub fn scrollbar_thumb_grab_offset(
    style: ScrollbarGeometryStyle,
    viewport_length: Pixels,
    metrics: ScrollbarMetrics,
    axis_position: Pixels,
) -> Option<Pixels> {
    let geometry = scrollbar_track_geometry(style, viewport_length, metrics)?;
    Some((axis_position - geometry.thumb_start).clamp(px(0.0), metrics.thumb_length))
}

/// Maps a dragged thumb pointer position back into a caller-owned scroll offset.
#[must_use]
pub fn scroll_offset_from_thumb_drag(
    style: ScrollbarGeometryStyle,
    viewport_length: Pixels,
    overflow_length: Pixels,
    metrics: ScrollbarMetrics,
    pointer_axis_position: Pixels,
    thumb_grab_offset: Pixels,
) -> Option<Pixels> {
    let geometry = scrollbar_track_geometry(style, viewport_length, metrics)?;
    let thumb_travel = (geometry.track_length - metrics.thumb_length).max(px(0.0));
    if thumb_travel <= px(0.0) {
        return Some(px(0.0));
    }

    let desired_thumb_offset = (pointer_axis_position - geometry.track_start - thumb_grab_offset)
        .clamp(px(0.0), thumb_travel);
    Some(overflow_length * (desired_thumb_offset / thumb_travel))
}
