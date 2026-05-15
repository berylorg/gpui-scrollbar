use gpui::px;
use gpui_scrollbar::{
    ScrollbarAxisHit, ScrollbarGeometryStyle, classify_scrollbar_axis_hit,
    scroll_offset_from_thumb_drag, scrollbar_metrics, scrollbar_thumb_grab_offset,
    scrollbar_track_geometry,
};

#[test]
fn scrollbar_metrics_hide_when_content_fits() {
    assert_eq!(
        scrollbar_metrics(
            ScrollbarGeometryStyle::default(),
            px(240.0),
            px(0.0),
            px(0.0)
        ),
        None
    );
}

#[test]
fn scrollbar_metrics_hide_when_track_has_no_length() {
    let style = ScrollbarGeometryStyle {
        track_inset: px(120.0),
        min_thumb_length: px(24.0),
    };

    assert_eq!(
        scrollbar_metrics(style, px(240.0), px(240.0), px(0.0)),
        None
    );
}

#[test]
fn scrollbar_metrics_move_the_thumb_as_scroll_advances() {
    let top = scrollbar_metrics(
        ScrollbarGeometryStyle::default(),
        px(240.0),
        px(240.0),
        px(0.0),
    )
    .expect("overflow should produce a visible scrollbar");
    let middle = scrollbar_metrics(
        ScrollbarGeometryStyle::default(),
        px(240.0),
        px(240.0),
        px(120.0),
    )
    .expect("overflow should produce a visible scrollbar");

    assert_eq!(top.thumb_length, middle.thumb_length);
    assert!(middle.thumb_offset > top.thumb_offset);
}

#[test]
fn scrollbar_metrics_clamp_scroll_offset_to_the_track() {
    let clamped = scrollbar_metrics(
        ScrollbarGeometryStyle::default(),
        px(200.0),
        px(100.0),
        px(999.0),
    )
    .expect("overflow should produce a visible scrollbar");
    let maxed = scrollbar_metrics(
        ScrollbarGeometryStyle::default(),
        px(200.0),
        px(100.0),
        px(100.0),
    )
    .expect("overflow should produce a visible scrollbar");

    assert_eq!(clamped, maxed);
}

#[test]
fn scrollbar_metrics_apply_configured_minimum_thumb_length() {
    let style = ScrollbarGeometryStyle {
        track_inset: px(8.0),
        min_thumb_length: px(32.0),
    };
    let metrics = scrollbar_metrics(style, px(240.0), px(10_000.0), px(0.0))
        .expect("overflow should produce a visible scrollbar");
    let geometry = scrollbar_track_geometry(style, px(240.0), metrics)
        .expect("visible scrollbar should have track geometry");

    assert_eq!(metrics.thumb_length, px(32.0));
    assert_eq!(geometry.track_start, px(8.0));
    assert_eq!(geometry.track_length, px(224.0));
}

#[test]
fn scrollbar_hit_classification_uses_current_thumb_bounds() {
    let metrics = scrollbar_metrics(
        ScrollbarGeometryStyle::default(),
        px(240.0),
        px(240.0),
        px(120.0),
    )
    .expect("overflow should produce a visible scrollbar");
    let geometry = scrollbar_track_geometry(ScrollbarGeometryStyle::default(), px(240.0), metrics)
        .expect("visible scrollbar should have track geometry");

    assert_eq!(
        classify_scrollbar_axis_hit(
            ScrollbarGeometryStyle::default(),
            px(240.0),
            metrics,
            geometry.thumb_start - px(1.0),
        ),
        Some(ScrollbarAxisHit::LaneBeforeThumb)
    );
    assert_eq!(
        classify_scrollbar_axis_hit(
            ScrollbarGeometryStyle::default(),
            px(240.0),
            metrics,
            geometry.thumb_start + px(1.0),
        ),
        Some(ScrollbarAxisHit::Thumb)
    );
    assert_eq!(
        classify_scrollbar_axis_hit(
            ScrollbarGeometryStyle::default(),
            px(240.0),
            metrics,
            geometry.thumb_end + px(1.0),
        ),
        Some(ScrollbarAxisHit::LaneAfterThumb)
    );
}

#[test]
fn scrollbar_thumb_grab_offset_clamps_to_the_thumb() {
    let metrics = scrollbar_metrics(
        ScrollbarGeometryStyle::default(),
        px(240.0),
        px(240.0),
        px(120.0),
    )
    .expect("overflow should produce a visible scrollbar");
    let geometry = scrollbar_track_geometry(ScrollbarGeometryStyle::default(), px(240.0), metrics)
        .expect("visible scrollbar should have track geometry");

    assert_eq!(
        scrollbar_thumb_grab_offset(
            ScrollbarGeometryStyle::default(),
            px(240.0),
            metrics,
            geometry.thumb_start - px(12.0),
        ),
        Some(px(0.0))
    );
    assert_eq!(
        scrollbar_thumb_grab_offset(
            ScrollbarGeometryStyle::default(),
            px(240.0),
            metrics,
            geometry.thumb_end + px(12.0),
        ),
        Some(metrics.thumb_length)
    );
}

#[test]
fn scrollbar_drag_mapping_preserves_pointer_grab_offset() {
    let metrics = scrollbar_metrics(
        ScrollbarGeometryStyle::default(),
        px(240.0),
        px(240.0),
        px(0.0),
    )
    .expect("overflow should produce a visible scrollbar");
    let geometry = scrollbar_track_geometry(ScrollbarGeometryStyle::default(), px(240.0), metrics)
        .expect("visible scrollbar should have track geometry");
    let grab_offset = px(10.0);
    let pointer = geometry.track_start + px(57.0) + grab_offset;

    assert_eq!(
        scroll_offset_from_thumb_drag(
            ScrollbarGeometryStyle::default(),
            px(240.0),
            px(240.0),
            metrics,
            pointer,
            grab_offset,
        ),
        Some(px(120.0))
    );
}

#[test]
fn scrollbar_drag_mapping_clamps_to_edges() {
    let metrics = scrollbar_metrics(
        ScrollbarGeometryStyle::default(),
        px(240.0),
        px(240.0),
        px(0.0),
    )
    .expect("overflow should produce a visible scrollbar");

    assert_eq!(
        scroll_offset_from_thumb_drag(
            ScrollbarGeometryStyle::default(),
            px(240.0),
            px(240.0),
            metrics,
            px(-100.0),
            px(8.0),
        ),
        Some(px(0.0))
    );
    assert_eq!(
        scroll_offset_from_thumb_drag(
            ScrollbarGeometryStyle::default(),
            px(240.0),
            px(240.0),
            metrics,
            px(999.0),
            px(8.0),
        ),
        Some(px(240.0))
    );
}
