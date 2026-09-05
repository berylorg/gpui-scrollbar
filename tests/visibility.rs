use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::prelude::*;
use gpui::{Bounds, Render, Window, div, point, px, size};
use gpui_scrollbar::{
    Axis, ScrollbarFadeConfig, ScrollbarInteraction, ScrollbarMountGeneration, ScrollbarOwnerId,
    ScrollbarOwnerKey, ScrollbarScrollState, ScrollbarState, ScrollbarStyle,
    ScrollbarVisibilityPolicy, render_scrollbar,
};

fn key(owner: u64, mount: u64) -> ScrollbarOwnerKey {
    ScrollbarOwnerKey::new(
        ScrollbarOwnerId::new(owner),
        ScrollbarMountGeneration::new(mount),
    )
}

fn assert_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 0.0001,
        "{actual} != {expected}"
    );
}

struct ObsoleteFrameDriverView {
    state: ScrollbarState,
    visibility: ScrollbarVisibilityPolicy,
    interaction: ScrollbarInteraction,
    owner: ScrollbarOwnerKey,
    replacement: ScrollbarOwnerKey,
    obsolete_during_render: bool,
    renders: Rc<Cell<usize>>,
}

impl Render for ObsoleteFrameDriverView {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get() + 1);
        let scrollbar = render_scrollbar(
            "obsolete-frame-driver",
            self.state.clone(),
            Axis::Vertical,
            ScrollbarStyle::default(),
            self.visibility.clone(),
            self.interaction.clone(),
        );
        assert_eq!(scrollbar.is_some(), self.obsolete_during_render);
        if self.obsolete_during_render {
            self.obsolete_during_render = false;
            assert!(
                self.state
                    .replace_owner(self.owner, self.replacement, window, cx)
            );
        }
        div().w(px(240.0)).h(px(240.0)).children(scrollbar)
    }
}

#[test]
fn managed_visibility_is_owner_keyed_and_sequences_are_monotonic() {
    let owner = key(1, 1);
    let state = ScrollbarState::new(owner);
    let now = Instant::now();
    assert_eq!(state.opacity_at(owner, now), Some(0.0));
    assert_eq!(state.opacity_at(key(2, 1), now), None);

    let first = state.record_activity_at(owner, now).unwrap();
    let second = state
        .record_activity_at(owner, now + Duration::from_millis(1))
        .unwrap();
    assert_eq!(first.owner, owner);
    assert_eq!(second.owner, owner);
    assert!(second.sequence > first.sequence);
}

#[test]
fn stale_sequence_cannot_advance_delay_or_fade() {
    let owner = key(1, 1);
    let state = ScrollbarState::new(owner);
    let config = ScrollbarFadeConfig::default();
    let start = Instant::now();
    let stale = state.record_activity_at(owner, start).unwrap();
    let current = state
        .record_activity_at(owner, start + Duration::from_millis(1))
        .unwrap();

    assert!(!state.advance_animation_at(
        stale,
        start + config.fade_duration + config.fade_delay,
        config,
    ));
    assert!(state.advance_animation_at(
        current,
        start + Duration::from_millis(1) + config.fade_duration,
        config,
    ));
}

#[test]
fn exact_current_sequence_completes_fade_in_delay_and_fade_out() {
    let owner = key(1, 1);
    let state = ScrollbarState::new(owner);
    let config = ScrollbarFadeConfig::default();
    let start = Instant::now();
    let current = state.record_activity_at(owner, start).unwrap();
    let fade_in_done = start + config.fade_duration;
    let fade_out_start = fade_in_done + config.fade_delay;

    assert!(state.advance_animation_at(current, fade_in_done, config));
    assert_near(state.opacity_at(owner, fade_in_done).unwrap(), 1.0);
    assert!(state.advance_animation_at(current, fade_out_start, config));
    assert!(
        state
            .opacity_at(owner, fade_out_start + config.fade_duration / 2)
            .unwrap()
            < 1.0
    );
    assert!(state.advance_animation_at(current, fade_out_start + config.fade_duration, config,));
    assert_near(
        state
            .opacity_at(owner, fade_out_start + config.fade_duration)
            .unwrap(),
        0.0,
    );
}

#[test]
fn new_activity_reverses_current_fade_without_opacity_jump() {
    let owner = key(1, 1);
    let state = ScrollbarState::new(owner);
    let config = ScrollbarFadeConfig::default();
    let start = Instant::now();
    let first = state.record_activity_at(owner, start).unwrap();
    let fade_in_done = start + config.fade_duration;
    let fade_out_start = fade_in_done + config.fade_delay;
    let midpoint = fade_out_start + config.fade_duration / 2;
    state.advance_animation_at(first, fade_in_done, config);
    state.advance_animation_at(first, fade_out_start, config);
    let before = state.opacity_at(owner, midpoint).unwrap();

    let second = state.record_activity_at(owner, midpoint).unwrap();
    assert!(second.sequence > first.sequence);
    assert_near(state.opacity_at(owner, midpoint).unwrap(), before);
    assert!(
        state
            .opacity_at(owner, midpoint + config.fade_duration / 2)
            .unwrap()
            > before
    );
}

#[test]
fn managed_visibility_remains_independent_per_retained_owner() {
    let first_owner = key(1, 1);
    let second_owner = key(2, 1);
    let first = ScrollbarState::new(first_owner);
    let second = ScrollbarState::new(second_owner);
    let now = Instant::now();
    first.record_activity_at(first_owner, now);

    assert!(
        first
            .opacity_at(first_owner, now + Duration::from_millis(90))
            .unwrap()
            > 0.0
    );
    assert_eq!(
        second.opacity_at(second_owner, now + Duration::from_millis(90)),
        Some(0.0)
    );
}

#[test]
fn direct_interaction_holds_visibility_until_exact_keyed_end() {
    let owner = key(1, 1);
    let state = ScrollbarState::new(owner);
    let config = ScrollbarFadeConfig::default();
    let start = Instant::now();
    let started = state.begin_direct_interaction_at(owner, start).unwrap();
    assert_near(state.opacity_at(owner, start).unwrap(), 1.0);
    assert!(!state.advance_animation_at(
        started,
        start + config.fade_delay + config.fade_duration,
        config,
    ));
    assert!(state.end_direct_interaction_at(key(2, 1), start).is_none());
    let ended = state
        .end_direct_interaction_at(owner, start + config.fade_duration)
        .unwrap();
    assert!(ended.sequence > started.sequence);
}

#[test]
fn managed_policy_rejects_stale_owner_and_frame_key() {
    let owner = key(1, 1);
    let state = ScrollbarState::new(owner);
    let config = ScrollbarFadeConfig::default();
    let start = Instant::now();
    let current = state.record_activity_at(owner, start).unwrap();
    let policy = state.managed_with_config(config, Rc::new(|_, _, _| {}));

    assert!(
        policy
            .opacity_for_owner_at(owner, true, start + config.fade_duration / 2)
            .is_some()
    );
    assert_eq!(policy.opacity_for_owner_at(key(2, 1), true, start), None);
    assert!(policy.should_request_animation_frame_for_key_at(current, true, start));

    state.record_activity_at(owner, start + Duration::from_millis(1));
    assert!(!policy.should_request_animation_frame_for_key_at(
        current,
        true,
        start + Duration::from_millis(1),
    ));
}

#[test]
fn always_visible_and_manual_variants_remain_explicit() {
    let owner = key(1, 1);
    let now = Instant::now();
    assert_eq!(
        ScrollbarVisibilityPolicy::always_visible().opacity_for_owner_at(owner, true, now),
        Some(1.0)
    );
    assert_eq!(
        ScrollbarVisibilityPolicy::always_visible().opacity_for_owner_at(owner, false, now),
        None
    );
    assert_eq!(
        ScrollbarVisibilityPolicy::manual_opacity(0.5).opacity_for_owner_at(owner, true, now),
        Some(0.5)
    );
    assert_eq!(
        ScrollbarVisibilityPolicy::manual_opacity(0.0).opacity_for_owner_at(owner, true, now),
        None
    );
}

#[gpui::test]
fn owner_replacement_and_unmount_obsolete_visibility_work(cx: &mut gpui::TestAppContext) {
    let cx = cx.add_empty_window();
    let owner = key(1, 1);
    let replacement = key(2, 1);
    let state = ScrollbarState::new(owner);
    let stale = state.record_activity_at(owner, Instant::now()).unwrap();

    cx.update(|window, cx| {
        assert!(state.replace_owner(owner, replacement, window, cx));
    });
    assert_ne!(state.visibility_key(), Some(stale));
    assert!(!state.advance_animation_at(stale, Instant::now(), Default::default()));

    cx.update(|window, cx| {
        assert!(state.unmount_viewport(replacement, window, cx));
    });
    assert_eq!(state.visibility_key(), None);
}

#[gpui::test]
fn late_managed_delay_and_deferred_repaint_are_obsolete_after_replacement(
    cx: &mut gpui::TestAppContext,
) {
    let cx = cx.add_empty_window();
    let owner = key(31, 7);
    let replacement = key(32, 1);
    let updates = Rc::new(std::cell::Cell::new(0));
    let state = ScrollbarState::new(owner);
    let policy = state.managed_with_config(
        ScrollbarFadeConfig {
            fade_delay: Duration::from_millis(5),
            fade_duration: Duration::from_millis(5),
        },
        {
            let updates = updates.clone();
            Rc::new(move |_, _, _| updates.set(updates.get() + 1))
        },
    );

    cx.update(|window, cx| {
        policy.record_viewport_activity(owner, window, cx);
        let current = state.visibility_key().expect("active frame key");
        assert!(state.replace_owner(owner, replacement, window, cx));
        assert!(!policy.request_animation_frame_for_key(current, window));
    });
    cx.executor().advance_clock(Duration::from_millis(20));
    cx.run_until_parked();

    assert_eq!(updates.get(), 0);
    assert_eq!(state.current_owner(), Some(replacement));
    assert_eq!(
        policy.opacity_for_owner_at(replacement, true, Instant::now()),
        None
    );
}

#[gpui::test]
fn mounted_render_frame_driver_cannot_resurrect_obsolete_visibility(cx: &mut gpui::TestAppContext) {
    let owner = key(41, 9);
    let replacement = key(42, 1);
    let state = ScrollbarState::new(owner);
    let stale = state
        .record_activity_at(owner, Instant::now())
        .expect("active visibility key");
    let updates = Rc::new(Cell::new(0));
    let visibility = state.managed_with_config(
        ScrollbarFadeConfig {
            fade_delay: Duration::from_secs(60),
            fade_duration: Duration::from_secs(60),
        },
        {
            let updates = updates.clone();
            Rc::new(move |_, _, _| updates.set(updates.get() + 1))
        },
    );
    let interaction = ScrollbarInteraction::new(
        move || {
            Some(ScrollbarScrollState {
                owner,
                viewport_bounds: Bounds::new(point(px(0.0), px(0.0)), size(px(240.0), px(240.0))),
                content_size: size(px(240.0), px(480.0)),
                scroll_offset: point(px(0.0), px(120.0)),
                page_distance: size(px(240.0), px(180.0)),
            })
        },
        |_, _| {},
        |_, _, _| {},
        |_| {},
        |_| {},
        |_, _, _| {},
    );
    let renders = Rc::new(Cell::new(0));
    #[cfg(feature = "test-support")]
    let probe = gpui_scrollbar::test_support::FrameDriverProbe::default();
    #[cfg(feature = "test-support")]
    let visibility = visibility.with_frame_driver_probe(probe.clone());
    let (_, cx) = cx.add_window_view({
        let state = state.clone();
        let renders = renders.clone();
        let visibility = visibility.clone();
        move |_, _| ObsoleteFrameDriverView {
            state,
            visibility,
            interaction,
            owner,
            replacement,
            obsolete_during_render: true,
            renders,
        }
    });
    let replacement_key = state.visibility_key().expect("replacement visibility key");

    assert_ne!(replacement_key, stale);
    assert_eq!(replacement_key.owner, replacement);
    assert!(replacement_key.sequence > stale.sequence);
    let renders_after_obsolete_driver = renders.get();
    assert!(renders_after_obsolete_driver >= 1);
    cx.run_until_parked();

    assert_eq!(updates.get(), 0);
    assert_eq!(renders.get(), renders_after_obsolete_driver);
    assert_eq!(state.visibility_key(), Some(replacement_key));
    #[cfg(feature = "test-support")]
    assert_eq!(
        probe.snapshot(),
        gpui_scrollbar::test_support::FrameDriverSnapshot {
            driver_calls: 1,
            last_driver_key: Some(stale),
            frame_requests: 0,
            last_requested_key: None,
        }
    );
    assert_eq!(
        visibility.opacity_for_owner_at(replacement, true, Instant::now()),
        None
    );
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn mounted_current_frame_driver_records_actual_frame_admission(cx: &mut gpui::TestAppContext) {
    struct CurrentFrameDriverView {
        state: ScrollbarState,
        visibility: ScrollbarVisibilityPolicy,
        interaction: ScrollbarInteraction,
    }

    impl Render for CurrentFrameDriverView {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            div().w(px(240.0)).h(px(240.0)).child(
                render_scrollbar(
                    "current-frame-driver",
                    self.state.clone(),
                    Axis::Vertical,
                    ScrollbarStyle::default(),
                    self.visibility.clone(),
                    self.interaction.clone(),
                )
                .expect("current fade renders its animation-frame driver"),
            )
        }
    }

    let owner = key(43, 1);
    let state = ScrollbarState::new(owner);
    let current = state.record_activity_at(owner, Instant::now()).unwrap();
    let probe = gpui_scrollbar::test_support::FrameDriverProbe::default();
    let visibility = state
        .managed_with_config(
            ScrollbarFadeConfig {
                fade_delay: Duration::from_secs(60),
                fade_duration: Duration::from_secs(60),
            },
            Rc::new(|_, _, _| {}),
        )
        .with_frame_driver_probe(probe.clone());
    let interaction = ScrollbarInteraction::new(
        move || {
            Some(ScrollbarScrollState {
                owner,
                viewport_bounds: Bounds::new(point(px(0.0), px(0.0)), size(px(240.0), px(240.0))),
                content_size: size(px(240.0), px(480.0)),
                scroll_offset: point(px(0.0), px(120.0)),
                page_distance: size(px(240.0), px(180.0)),
            })
        },
        |_, _| {},
        |_, _, _| {},
        |_| {},
        |_| {},
        |_, _, _| {},
    );
    let (_, cx) = cx.add_window_view(move |_, _| CurrentFrameDriverView {
        state,
        visibility,
        interaction,
    });
    cx.run_until_parked();

    let snapshot = probe.snapshot();
    assert!(snapshot.driver_calls > 0);
    assert_eq!(snapshot.last_driver_key, Some(current));
    assert_eq!(snapshot.frame_requests, snapshot.driver_calls);
    assert_eq!(snapshot.last_requested_key, Some(current));
}
