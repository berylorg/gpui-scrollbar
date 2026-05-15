use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{App, Task, Window};

/// Callback invoked when managed scrollbar visibility changes need an owner repaint.
///
/// The callback usually notifies the view that owns the scrollable region. It
/// is deferred by this crate before invocation so viewport owners may report
/// activity from their own update handlers without re-entering themselves. It
/// does not own scroll state and must not install key, wheel, or focus routing.
pub type ScrollbarVisibilityUpdateCallback = Rc<dyn Fn(&mut Window, &mut App)>;

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

/// Retained managed visibility state for one scroll region.
#[derive(Clone)]
pub struct ScrollbarVisibilityState {
    inner: Rc<RefCell<ScrollbarVisibilityStateInner>>,
}

struct ScrollbarVisibilityStateInner {
    generation: u64,
    last_activity_at: Option<Instant>,
    transition: Option<ScrollbarVisibilityTransition>,
    animation_task: Option<Task<()>>,
    direct_interaction_count: u32,
}

#[derive(Clone, Copy)]
struct ScrollbarVisibilityTransition {
    started_at: Instant,
    from_opacity: f32,
    to_opacity: f32,
}

/// Explicit visibility policy used by rendered scrollbar chrome.
#[derive(Clone)]
pub enum ScrollbarVisibilityPolicy {
    /// Use managed auto-fading visibility state.
    Managed(ManagedScrollbarVisibility),
    /// Keep scrollbar chrome fully opaque whenever the scroll geometry overflows.
    AlwaysVisible,
    /// Use an explicit caller-provided opacity.
    ManualOpacity(f32),
}

/// Cloneable managed visibility policy data.
#[derive(Clone)]
pub struct ManagedScrollbarVisibility {
    state: ScrollbarVisibilityState,
    config: ScrollbarFadeConfig,
    on_update: ScrollbarVisibilityUpdateCallback,
}

impl Default for ScrollbarVisibilityState {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollbarVisibilityState {
    /// Creates idle managed visibility state for one scroll region.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(ScrollbarVisibilityStateInner {
                generation: 0,
                last_activity_at: None,
                transition: None,
                animation_task: None,
                direct_interaction_count: 0,
            })),
        }
    }

    /// Builds the ordinary managed visibility policy with default timing.
    #[must_use]
    pub fn managed(
        &self,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) -> ScrollbarVisibilityPolicy {
        ScrollbarVisibilityPolicy::managed(self.clone(), on_update)
    }

    /// Builds a managed visibility policy with explicit timing.
    #[must_use]
    pub fn managed_with_config(
        &self,
        config: ScrollbarFadeConfig,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) -> ScrollbarVisibilityPolicy {
        ScrollbarVisibilityPolicy::managed_with_config(self.clone(), config, on_update)
    }

    /// Records viewport-originated activity with default timing.
    pub fn record_viewport_activity(
        &self,
        window: &mut Window,
        cx: &mut App,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) {
        self.record_viewport_activity_with_config(
            ScrollbarFadeConfig::default(),
            window,
            cx,
            on_update,
        );
    }

    /// Records viewport-originated activity with explicit timing.
    pub fn record_viewport_activity_with_config(
        &self,
        config: ScrollbarFadeConfig,
        window: &mut Window,
        cx: &mut App,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) {
        self.record_activity_with_config(config, window, cx, on_update);
    }

    /// Returns the current managed opacity.
    #[must_use]
    pub fn opacity(&self) -> f32 {
        self.opacity_at(Instant::now())
    }

    /// Returns managed opacity at a specific instant.
    #[must_use]
    pub fn opacity_at(&self, now: Instant) -> f32 {
        self.inner
            .borrow()
            .opacity_at(now, ScrollbarFadeConfig::default())
    }

    fn opacity_at_with_config(&self, now: Instant, config: ScrollbarFadeConfig) -> f32 {
        self.inner.borrow().opacity_at(now, config)
    }

    fn is_animating_at_with_config(&self, now: Instant, config: ScrollbarFadeConfig) -> bool {
        self.inner.borrow().is_animating_at(now, config)
    }

    /// Returns true while a fade transition is actively changing opacity.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.is_animating_at(Instant::now())
    }

    /// Returns true at a specific instant while a fade transition is active.
    #[must_use]
    pub fn is_animating_at(&self, now: Instant) -> bool {
        self.inner
            .borrow()
            .is_animating_at(now, ScrollbarFadeConfig::default())
    }

    /// Records managed activity at a specific instant.
    ///
    /// This method intentionally does not schedule UI invalidation. Use
    /// [`record_viewport_activity`](Self::record_viewport_activity) in live UI
    /// code.
    pub fn record_activity_at(&self, now: Instant) {
        self.inner
            .borrow_mut()
            .record_activity_at(now, ScrollbarFadeConfig::default());
    }

    /// Marks the start of a scrollbar direct interaction at a specific instant.
    pub fn begin_direct_interaction_at(&self, now: Instant) {
        let mut inner = self.inner.borrow_mut();
        inner.direct_interaction_count = inner.direct_interaction_count.saturating_add(1);
        inner.record_activity_at(now, ScrollbarFadeConfig::default());
    }

    /// Marks the end of a scrollbar direct interaction at a specific instant.
    pub fn end_direct_interaction_at(&self, now: Instant) {
        let mut inner = self.inner.borrow_mut();
        inner.direct_interaction_count = inner.direct_interaction_count.saturating_sub(1);
        inner.last_activity_at = Some(now);
    }

    /// Advances managed fade state at a specific instant.
    ///
    /// Returns true when the owner should repaint.
    pub fn advance_animation_at(&self, now: Instant, config: ScrollbarFadeConfig) -> bool {
        self.inner.borrow_mut().advance_animation_at(now, config)
    }

    pub(crate) fn record_activity_with_config(
        &self,
        config: ScrollbarFadeConfig,
        window: &mut Window,
        cx: &mut App,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) {
        let (generation, should_update) = {
            let mut inner = self.inner.borrow_mut();
            let generation = inner.record_activity_at(Instant::now(), config);
            (generation, inner.should_update_after_activity())
        };
        if should_update {
            defer_update(window, cx, on_update.clone());
        }
        window.refresh();
        self.schedule_animation(config, generation, window, cx, on_update);
    }

    pub(crate) fn begin_direct_interaction_with_config(
        &self,
        config: ScrollbarFadeConfig,
        window: &mut Window,
        cx: &mut App,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) {
        let generation = {
            let mut inner = self.inner.borrow_mut();
            inner.direct_interaction_count = inner.direct_interaction_count.saturating_add(1);
            inner.record_activity_at(Instant::now(), config)
        };
        defer_update(window, cx, on_update.clone());
        window.refresh();
        self.schedule_animation(config, generation, window, cx, on_update);
    }

    pub(crate) fn end_direct_interaction(&self) {
        self.end_direct_interaction_at(Instant::now());
    }

    fn schedule_animation(
        &self,
        config: ScrollbarFadeConfig,
        generation: u64,
        window: &mut Window,
        cx: &mut App,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) {
        let Some(next_delay) = self.next_animation_delay_at(Instant::now(), config, generation)
        else {
            return;
        };

        let state = self.clone();
        let update_callback = on_update.clone();
        let animation_task = window.spawn(cx, async move |cx| {
            cx.background_executor().timer(next_delay).await;
            let _ = cx.update(move |window, cx| {
                state.advance_animation(config, generation, window, cx, update_callback);
            });
        });

        if let Ok(mut inner) = self.inner.try_borrow_mut() {
            inner.animation_task = Some(animation_task);
        }
    }

    fn next_animation_delay_at(
        &self,
        now: Instant,
        config: ScrollbarFadeConfig,
        generation: u64,
    ) -> Option<Duration> {
        let inner = self.inner.borrow();
        if inner.generation != generation {
            return None;
        }
        inner.next_animation_delay_at(now, config)
    }

    fn advance_animation(
        &self,
        config: ScrollbarFadeConfig,
        generation: u64,
        window: &mut Window,
        cx: &mut App,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) {
        let should_update = {
            let mut inner = self.inner.borrow_mut();
            if inner.generation != generation {
                return;
            }
            inner.animation_task = None;
            inner.advance_animation_at(Instant::now(), config)
        };

        if should_update {
            defer_update(window, cx, on_update.clone());
            window.refresh();
        }

        if self.inner.borrow().generation == generation {
            self.schedule_animation(config, generation, window, cx, on_update);
        }
    }
}

fn defer_update(window: &mut Window, cx: &mut App, on_update: ScrollbarVisibilityUpdateCallback) {
    window.defer(cx, move |window, cx| {
        on_update(window, cx);
    });
}

impl ScrollbarVisibilityPolicy {
    /// Creates managed auto-fading visibility with default timing.
    #[must_use]
    pub fn managed(
        state: ScrollbarVisibilityState,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) -> Self {
        Self::Managed(ManagedScrollbarVisibility {
            state,
            config: ScrollbarFadeConfig::default(),
            on_update,
        })
    }

    /// Creates managed auto-fading visibility with explicit timing.
    #[must_use]
    pub fn managed_with_config(
        state: ScrollbarVisibilityState,
        config: ScrollbarFadeConfig,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) -> Self {
        Self::Managed(ManagedScrollbarVisibility {
            state,
            config,
            on_update,
        })
    }

    /// Creates a policy that is fully visible whenever scroll geometry overflows.
    #[must_use]
    pub const fn always_visible() -> Self {
        Self::AlwaysVisible
    }

    /// Creates a policy with explicit caller-provided opacity.
    #[must_use]
    pub const fn manual_opacity(opacity: f32) -> Self {
        Self::ManualOpacity(opacity)
    }

    /// Returns the opacity to render when the geometry overflows.
    #[must_use]
    pub fn opacity_for_overflow(&self, has_overflow: bool) -> Option<f32> {
        self.opacity_for_overflow_at(has_overflow, Instant::now())
    }

    /// Returns the opacity to render at a specific instant.
    #[must_use]
    pub fn opacity_for_overflow_at(&self, has_overflow: bool, now: Instant) -> Option<f32> {
        if !has_overflow {
            return None;
        }
        let opacity = match self {
            Self::Managed(managed) => managed.state.opacity_at_with_config(now, managed.config),
            Self::AlwaysVisible => 1.0,
            Self::ManualOpacity(opacity) => *opacity,
        }
        .clamp(0.0, 1.0);
        (opacity > 0.0).then_some(opacity)
    }

    /// Returns true when rendering should request a presentation animation
    /// frame for managed visibility.
    ///
    /// Ordinary callers do not need this because the rendering helpers drive
    /// animation frames themselves. The method is exposed for custom renderers
    /// that still want the crate-owned visibility lifecycle.
    #[must_use]
    pub fn should_request_animation_frame_for_overflow(&self, has_overflow: bool) -> bool {
        self.should_request_animation_frame_for_overflow_at(has_overflow, Instant::now())
    }

    /// Returns true at a specific instant when rendering should request a
    /// presentation animation frame for managed visibility.
    #[must_use]
    pub fn should_request_animation_frame_for_overflow_at(
        &self,
        has_overflow: bool,
        now: Instant,
    ) -> bool {
        if !has_overflow {
            return false;
        }

        match self {
            Self::Managed(managed) => managed
                .state
                .is_animating_at_with_config(now, managed.config),
            Self::AlwaysVisible | Self::ManualOpacity(_) => false,
        }
    }

    pub(crate) fn request_animation_frame_for_overflow(
        &self,
        has_overflow: bool,
        window: &mut Window,
    ) {
        if self.should_request_animation_frame_for_overflow(has_overflow) {
            window.request_animation_frame();
        }
    }

    /// Records viewport-originated activity for managed visibility policies.
    pub fn record_viewport_activity(&self, window: &mut Window, cx: &mut App) {
        if let Self::Managed(managed) = self {
            managed.record_viewport_activity(window, cx);
        }
    }

    pub(crate) fn record_direct_activity(&self, window: &mut Window, cx: &mut App) {
        if let Self::Managed(managed) = self {
            managed.record_direct_activity(window, cx);
        }
    }

    pub(crate) fn begin_direct_interaction(&self, window: &mut Window, cx: &mut App) {
        if let Self::Managed(managed) = self {
            managed.begin_direct_interaction(window, cx);
        }
    }

    pub(crate) fn end_direct_interaction(&self) {
        if let Self::Managed(managed) = self {
            managed.state.end_direct_interaction();
        }
    }
}

impl ManagedScrollbarVisibility {
    /// Returns the retained state for this managed policy.
    #[must_use]
    pub fn state(&self) -> ScrollbarVisibilityState {
        self.state.clone()
    }

    /// Records viewport-originated activity.
    pub fn record_viewport_activity(&self, window: &mut Window, cx: &mut App) {
        self.state
            .record_activity_with_config(self.config, window, cx, self.on_update.clone());
    }

    fn record_direct_activity(&self, window: &mut Window, cx: &mut App) {
        self.record_viewport_activity(window, cx);
    }

    fn begin_direct_interaction(&self, window: &mut Window, cx: &mut App) {
        self.state.begin_direct_interaction_with_config(
            self.config,
            window,
            cx,
            self.on_update.clone(),
        );
    }
}

impl ScrollbarVisibilityStateInner {
    fn record_activity_at(&mut self, now: Instant, config: ScrollbarFadeConfig) -> u64 {
        let current_opacity = self.opacity_at(now, config);
        self.generation = self.generation.saturating_add(1);
        self.last_activity_at = Some(now);
        self.transition = if current_opacity >= (1.0 - f32::EPSILON) {
            None
        } else {
            Some(ScrollbarVisibilityTransition {
                started_at: now,
                from_opacity: current_opacity,
                to_opacity: 1.0,
            })
        };
        self.animation_task = None;
        self.generation
    }

    fn should_update_after_activity(&self) -> bool {
        self.transition.is_some()
    }

    fn opacity_at(&self, now: Instant, config: ScrollbarFadeConfig) -> f32 {
        if let Some(transition) = &self.transition {
            transition.opacity(now, config)
        } else if self.last_activity_at.is_some() || self.direct_interaction_count > 0 {
            1.0
        } else {
            0.0
        }
    }

    fn is_animating_at(&self, now: Instant, config: ScrollbarFadeConfig) -> bool {
        self.transition
            .as_ref()
            .is_some_and(|transition| transition.is_active(now, config))
    }

    fn next_animation_delay_at(
        &self,
        now: Instant,
        config: ScrollbarFadeConfig,
    ) -> Option<Duration> {
        if let Some(transition) = &self.transition {
            return Some(transition.remaining_duration(now, config));
        }

        let last_activity_at = self.last_activity_at?;
        let fade_deadline = last_activity_at + config.fade_delay;
        Some(fade_deadline.saturating_duration_since(now))
    }

    fn advance_animation_at(&mut self, now: Instant, config: ScrollbarFadeConfig) -> bool {
        if let Some(transition) = &self.transition {
            if transition.is_active(now, config) {
                return false;
            }
            let target_opacity = transition.to_opacity;
            self.transition = None;
            if target_opacity <= 0.0 {
                self.last_activity_at = None;
            }
            return true;
        }

        let Some(last_activity_at) = self.last_activity_at else {
            return false;
        };
        let fade_deadline = last_activity_at + config.fade_delay;
        if now < fade_deadline {
            return false;
        }
        if self.direct_interaction_count > 0 {
            self.last_activity_at = Some(now);
            return false;
        }

        let current_opacity = self.opacity_at(now, config);
        if current_opacity <= 0.0 {
            self.last_activity_at = None;
            return false;
        }

        self.transition = Some(ScrollbarVisibilityTransition {
            started_at: now,
            from_opacity: current_opacity,
            to_opacity: 0.0,
        });
        true
    }
}

impl ScrollbarVisibilityTransition {
    fn duration(&self, config: ScrollbarFadeConfig) -> Duration {
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

    fn progress(&self, now: Instant, config: ScrollbarFadeConfig) -> f32 {
        let duration = self.duration(config);
        if duration.is_zero() {
            return 1.0;
        }

        let elapsed = now.saturating_duration_since(self.started_at);
        (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
    }

    fn opacity(&self, now: Instant, config: ScrollbarFadeConfig) -> f32 {
        let progress = self.progress(now, config);
        let eased_progress = progress * progress * (3.0 - (2.0 * progress));
        self.from_opacity + ((self.to_opacity - self.from_opacity) * eased_progress)
    }

    fn is_active(&self, now: Instant, config: ScrollbarFadeConfig) -> bool {
        self.progress(now, config) < 1.0
    }

    fn remaining_duration(&self, now: Instant, config: ScrollbarFadeConfig) -> Duration {
        let duration = self.duration(config);
        let elapsed = now.saturating_duration_since(self.started_at);
        duration.saturating_sub(elapsed)
    }
}
