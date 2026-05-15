use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui_scrollbar::{ScrollbarFadeConfig, ScrollbarVisibilityPolicy, ScrollbarVisibilityState};

fn assert_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 0.0001,
        "expected {actual} to be near {expected}"
    );
}

#[test]
fn managed_visibility_is_hidden_until_activity() {
    let state = ScrollbarVisibilityState::new();
    let now = Instant::now();

    assert_eq!(state.opacity_at(now), 0.0);

    state.record_activity_at(now);

    assert_eq!(state.opacity_at(now), 0.0);
    assert!(state.opacity_at(now + Duration::from_millis(90)) > 0.0);
}

#[test]
fn managed_visibility_fades_out_after_idle_delay() {
    let state = ScrollbarVisibilityState::new();
    let config = ScrollbarFadeConfig::default();
    let start = Instant::now();
    let fade_in_done = start + config.fade_duration;
    let fade_out_start = fade_in_done + config.fade_delay;

    state.record_activity_at(start);
    assert!(state.advance_animation_at(fade_in_done, config));
    assert_near(state.opacity_at(fade_in_done), 1.0);

    assert!(state.advance_animation_at(fade_out_start, config));
    assert_near(state.opacity_at(fade_out_start), 1.0);
    assert!(state.opacity_at(fade_out_start + config.fade_duration / 2) < 1.0);

    assert!(state.advance_animation_at(fade_out_start + config.fade_duration, config));
    assert_near(state.opacity_at(fade_out_start + config.fade_duration), 0.0);
}

#[test]
fn activity_during_fade_reverses_without_opacity_jump() {
    let state = ScrollbarVisibilityState::new();
    let config = ScrollbarFadeConfig::default();
    let start = Instant::now();
    let fade_in_done = start + config.fade_duration;
    let fade_out_start = fade_in_done + config.fade_delay;
    let fade_out_midpoint = fade_out_start + config.fade_duration / 2;

    state.record_activity_at(start);
    state.advance_animation_at(fade_in_done, config);
    state.advance_animation_at(fade_out_start, config);

    let opacity_before_activity = state.opacity_at(fade_out_midpoint);
    state.record_activity_at(fade_out_midpoint);
    let opacity_after_activity = state.opacity_at(fade_out_midpoint);

    assert_near(opacity_after_activity, opacity_before_activity);
    assert!(
        state.opacity_at(fade_out_midpoint + config.fade_duration / 2) > opacity_after_activity
    );
}

#[test]
fn direct_interaction_prevents_idle_fade_until_ended() {
    let state = ScrollbarVisibilityState::new();
    let config = ScrollbarFadeConfig::default();
    let start = Instant::now();
    let fade_in_done = start + config.fade_duration;
    let held_after_deadline = fade_in_done + config.fade_delay + Duration::from_millis(50);
    let release = held_after_deadline + Duration::from_millis(10);
    let fade_after_release = release + config.fade_delay;

    state.begin_direct_interaction_at(start);
    state.advance_animation_at(fade_in_done, config);
    assert!(!state.advance_animation_at(held_after_deadline, config));
    assert_near(state.opacity_at(held_after_deadline), 1.0);

    state.end_direct_interaction_at(release);
    assert!(state.advance_animation_at(fade_after_release, config));
    assert!(state.opacity_at(fade_after_release + config.fade_duration / 2) < 1.0);
}

#[test]
fn managed_visibility_state_is_per_region() {
    let first = ScrollbarVisibilityState::new();
    let second = ScrollbarVisibilityState::new();
    let now = Instant::now();

    first.record_activity_at(now);

    assert!(first.opacity_at(now + Duration::from_millis(90)) > 0.0);
    assert_eq!(second.opacity_at(now + Duration::from_millis(90)), 0.0);
}

#[test]
fn policy_hides_when_no_overflow_even_after_activity() {
    let state = ScrollbarVisibilityState::new();
    let now = Instant::now();
    state.record_activity_at(now);
    let policy = state.managed(Rc::new(|_, _| {}));

    assert_eq!(
        policy.opacity_for_overflow_at(false, now + Duration::from_millis(90)),
        None
    );
}

#[test]
fn always_visible_and_manual_opacity_are_explicit_policies() {
    let now = Instant::now();

    assert_eq!(
        ScrollbarVisibilityPolicy::always_visible().opacity_for_overflow_at(true, now),
        Some(1.0)
    );
    assert_eq!(
        ScrollbarVisibilityPolicy::always_visible().opacity_for_overflow_at(false, now),
        None
    );
    assert_eq!(
        ScrollbarVisibilityPolicy::manual_opacity(0.5).opacity_for_overflow_at(true, now),
        Some(0.5)
    );
    assert_eq!(
        ScrollbarVisibilityPolicy::manual_opacity(2.0).opacity_for_overflow_at(true, now),
        Some(1.0)
    );
    assert_eq!(
        ScrollbarVisibilityPolicy::manual_opacity(0.0).opacity_for_overflow_at(true, now),
        None
    );
}

#[test]
fn managed_policy_requests_animation_frames_only_while_transitioning() {
    let state = ScrollbarVisibilityState::new();
    let config = ScrollbarFadeConfig::default();
    let start = Instant::now();
    let fade_in_midpoint = start + config.fade_duration / 2;
    let fade_in_done = start + config.fade_duration;
    let fade_out_start = fade_in_done + config.fade_delay;
    let fade_out_midpoint = fade_out_start + config.fade_duration / 2;
    let fade_out_done = fade_out_start + config.fade_duration;
    let policy = state.managed_with_config(config, Rc::new(|_, _| {}));

    state.record_activity_at(start);

    assert!(policy.should_request_animation_frame_for_overflow_at(true, start));
    assert!(policy.should_request_animation_frame_for_overflow_at(true, fade_in_midpoint));
    assert!(!policy.should_request_animation_frame_for_overflow_at(true, fade_in_done));
    assert!(!policy.should_request_animation_frame_for_overflow_at(false, fade_in_midpoint));

    assert!(state.advance_animation_at(fade_in_done, config));
    assert!(!policy.should_request_animation_frame_for_overflow_at(true, fade_in_done));

    assert!(state.advance_animation_at(fade_out_start, config));

    assert!(policy.should_request_animation_frame_for_overflow_at(true, fade_out_start));
    assert!(policy.should_request_animation_frame_for_overflow_at(true, fade_out_midpoint));
    assert!(!policy.should_request_animation_frame_for_overflow_at(true, fade_out_done));
}

#[test]
fn repeated_activity_keeps_animation_frame_driving_continuous() {
    let state = ScrollbarVisibilityState::new();
    let config = ScrollbarFadeConfig::default();
    let start = Instant::now();
    let fade_out_start = start + config.fade_duration + config.fade_delay;
    let fade_out_midpoint = fade_out_start + config.fade_duration / 2;
    let fade_back_in_midpoint = fade_out_midpoint + config.fade_duration / 2;
    let policy = state.managed_with_config(config, Rc::new(|_, _| {}));

    state.record_activity_at(start);
    state.advance_animation_at(start + config.fade_duration, config);
    state.advance_animation_at(fade_out_start, config);
    assert!(policy.should_request_animation_frame_for_overflow_at(true, fade_out_midpoint));

    state.record_activity_at(fade_out_midpoint);

    assert!(policy.should_request_animation_frame_for_overflow_at(true, fade_out_midpoint));
    assert!(policy.should_request_animation_frame_for_overflow_at(
        true,
        fade_out_midpoint + Duration::from_millis(10)
    ));
    assert!(!policy.should_request_animation_frame_for_overflow_at(true, fade_back_in_midpoint));
}

#[test]
fn direct_interaction_holds_visibility_without_unneeded_animation_frames() {
    let state = ScrollbarVisibilityState::new();
    let config = ScrollbarFadeConfig::default();
    let start = Instant::now();
    let fade_in_done = start + config.fade_duration;
    let held_after_deadline = fade_in_done + config.fade_delay + Duration::from_millis(50);
    let release = held_after_deadline + Duration::from_millis(10);
    let fade_out_start = release + config.fade_delay;
    let fade_out_midpoint = fade_out_start + config.fade_duration / 2;
    let policy = state.managed_with_config(config, Rc::new(|_, _| {}));

    state.begin_direct_interaction_at(start);

    assert!(!policy.should_request_animation_frame_for_overflow_at(true, start));
    assert_near(policy.opacity_for_overflow_at(true, start).unwrap(), 1.0);

    assert!(!state.advance_animation_at(fade_in_done, config));
    assert!(!state.advance_animation_at(held_after_deadline, config));
    assert!(!policy.should_request_animation_frame_for_overflow_at(true, held_after_deadline));

    state.end_direct_interaction_at(release);
    assert!(state.advance_animation_at(fade_out_start, config));

    assert!(policy.should_request_animation_frame_for_overflow_at(true, fade_out_start));
    assert!(policy.should_request_animation_frame_for_overflow_at(true, fade_out_midpoint));
}
