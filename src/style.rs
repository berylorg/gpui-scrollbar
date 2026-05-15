use gpui::{Pixels, px};

use crate::ScrollbarGeometryStyle;

/// App-neutral visual and hit-target styling for rendered scrollbars.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarStyle {
    /// Geometry used to compute the track and thumb position.
    pub geometry: ScrollbarGeometryStyle,
    /// Visible thumb thickness on the cross axis.
    pub thickness: Pixels,
    /// Invisible pointer lane thickness used for clicking and dragging.
    pub hit_lane_thickness: Pixels,
    /// Thumb color as `0xRRGGBB`.
    pub thumb_color: u32,
}

impl Default for ScrollbarStyle {
    fn default() -> Self {
        Self {
            geometry: ScrollbarGeometryStyle::default(),
            thickness: px(4.0),
            hit_lane_thickness: px(18.0),
            thumb_color: 0x94a3b8,
        }
    }
}

impl ScrollbarStyle {
    pub(crate) fn normalized(self) -> Self {
        Self {
            geometry: self.geometry.normalized(),
            thickness: self.thickness.max(px(0.0)),
            hit_lane_thickness: self.hit_lane_thickness.max(px(0.0)),
            thumb_color: self.thumb_color,
        }
    }
}
