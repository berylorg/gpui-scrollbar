use std::time::{Duration, Instant};

use gpui::Task;

use crate::{ScrollbarOwnerKey, ScrollbarVisibilityKey, ScrollbarVisibilitySequence};

/// Timing used by managed scrollbar visibility.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScrollbarFadeConfig {
    /// How long the scrollbar remains fully visible after activity.
    pub fade_delay: Duration,
    /// How long the fade transition lasts.
    pub fade_duration: Duration,
}

impl Default for ScrollbarFadeConfig {
    fn default() -> Self {
        Self {
            fade_delay: Duration::from_secs(2),
            fade_duration: Duration::from_millis(180),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ScrollbarVisibilityTransition {
    started_at: Instant,
    from_opacity: f32,
    to_opacity: f32,
}

pub(crate) struct ScrollbarVisibilityData {
    pub(crate) sequence: ScrollbarVisibilitySequence,
    pub(crate) last_activity_at: Option<Instant>,
    pub(crate) transition: Option<ScrollbarVisibilityTransition>,
    pub(crate) animation_task: Option<Task<()>>,
    pub(crate) direct_interaction: bool,
}

impl ScrollbarVisibilityData {
    pub(crate) fn new() -> Self {
        Self {
            sequence: ScrollbarVisibilitySequence::INITIAL,
            last_activity_at: None,
            transition: None,
            animation_task: None,
            direct_interaction: false,
        }
    }

    pub(crate) fn invalidate(&mut self) {
        self.sequence = self.sequence.next();
        self.last_activity_at = None;
        self.transition = None;
        self.animation_task = None;
        self.direct_interaction = false;
    }

    pub(crate) fn key(&self, owner: ScrollbarOwnerKey) -> ScrollbarVisibilityKey {
        ScrollbarVisibilityKey {
            owner,
            sequence: self.sequence,
        }
    }

    pub(crate) fn record_activity_at(
        &mut self,
        owner: ScrollbarOwnerKey,
        now: Instant,
        config: ScrollbarFadeConfig,
    ) -> ScrollbarVisibilityKey {
        let current_opacity = self.opacity_at(now, config);
        self.sequence = self.sequence.next();
        self.last_activity_at = Some(now);
        self.transition =
            (current_opacity < 1.0 - f32::EPSILON).then_some(ScrollbarVisibilityTransition {
                started_at: now,
                from_opacity: current_opacity,
                to_opacity: 1.0,
            });
        self.animation_task = None;
        self.key(owner)
    }

    pub(crate) fn opacity_at(&self, now: Instant, config: ScrollbarFadeConfig) -> f32 {
        if let Some(transition) = self.transition {
            transition.opacity(now, config)
        } else if self.last_activity_at.is_some() || self.direct_interaction {
            1.0
        } else {
            0.0
        }
    }

    pub(crate) fn is_animating_at(&self, now: Instant, config: ScrollbarFadeConfig) -> bool {
        self.transition
            .is_some_and(|transition| transition.is_active(now, config))
    }

    pub(crate) fn next_animation_delay_at(
        &self,
        now: Instant,
        config: ScrollbarFadeConfig,
    ) -> Option<Duration> {
        if let Some(transition) = self.transition {
            return Some(transition.remaining_duration(now, config));
        }
        let last_activity_at = self.last_activity_at?;
        Some((last_activity_at + config.fade_delay).saturating_duration_since(now))
    }

    pub(crate) fn advance_animation_at(
        &mut self,
        now: Instant,
        config: ScrollbarFadeConfig,
    ) -> bool {
        if let Some(transition) = self.transition {
            if transition.is_active(now, config) {
                return false;
            }
            self.transition = None;
            if transition.to_opacity <= 0.0 {
                self.last_activity_at = None;
            }
            return true;
        }
        let Some(last_activity_at) = self.last_activity_at else {
            return false;
        };
        if now < last_activity_at + config.fade_delay {
            return false;
        }
        if self.direct_interaction {
            self.last_activity_at = Some(now);
            return false;
        }
        self.transition = Some(ScrollbarVisibilityTransition {
            started_at: now,
            from_opacity: self.opacity_at(now, config),
            to_opacity: 0.0,
        });
        true
    }
}

impl ScrollbarVisibilityTransition {
    fn duration(self, config: ScrollbarFadeConfig) -> Duration {
        let delta = (self.to_opacity - self.from_opacity).abs();
        if delta <= f32::EPSILON {
            return Duration::ZERO;
        }
        let duration = config.fade_duration.mul_f32(delta);
        if duration.is_zero() {
            Duration::from_millis(1)
        } else {
            duration
        }
    }

    fn progress(self, now: Instant, config: ScrollbarFadeConfig) -> f32 {
        let duration = self.duration(config);
        if duration.is_zero() {
            return 1.0;
        }
        let elapsed = now.saturating_duration_since(self.started_at);
        (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
    }

    fn opacity(self, now: Instant, config: ScrollbarFadeConfig) -> f32 {
        let progress = self.progress(now, config);
        let eased = progress * progress * (3.0 - 2.0 * progress);
        self.from_opacity + (self.to_opacity - self.from_opacity) * eased
    }

    fn is_active(self, now: Instant, config: ScrollbarFadeConfig) -> bool {
        self.progress(now, config) < 1.0
    }

    fn remaining_duration(self, now: Instant, config: ScrollbarFadeConfig) -> Duration {
        self.duration(config)
            .saturating_sub(now.saturating_duration_since(self.started_at))
    }
}
