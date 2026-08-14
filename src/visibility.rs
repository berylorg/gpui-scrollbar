use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{App, Window};

pub use crate::visibility_state::ScrollbarFadeConfig;
use crate::{ScrollbarOwnerKey, ScrollbarState, ScrollbarVisibilityKey};

/// Keyed callback invoked when managed visibility needs an owner repaint.
pub type ScrollbarVisibilityUpdateCallback =
    Rc<dyn Fn(ScrollbarVisibilityKey, &mut Window, &mut App)>;

impl ScrollbarVisibilityPolicy {
    /// Creates keyed managed auto-fading visibility with default timing.
    #[must_use]
    pub fn managed(state: ScrollbarState, on_update: ScrollbarVisibilityUpdateCallback) -> Self {
        Self::managed_with_config(state, ScrollbarFadeConfig::default(), on_update)
    }

    /// Creates keyed managed auto-fading visibility with explicit timing.
    #[must_use]
    pub fn managed_with_config(
        state: ScrollbarState,
        config: ScrollbarFadeConfig,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) -> Self {
        Self::Managed(ManagedScrollbarVisibility {
            state,
            config,
            on_update,
        })
    }

    /// Creates a policy fully visible whenever geometry overflows.
    #[must_use]
    pub const fn always_visible() -> Self {
        Self::AlwaysVisible
    }

    /// Creates a policy with explicit caller-provided opacity.
    #[must_use]
    pub const fn manual_opacity(opacity: f32) -> Self {
        Self::ManualOpacity(opacity)
    }

    /// Returns opacity under the exact mounted owner key.
    #[must_use]
    pub fn opacity_for_owner_at(
        &self,
        owner: ScrollbarOwnerKey,
        has_overflow: bool,
        now: Instant,
    ) -> Option<f32> {
        if !has_overflow {
            return None;
        }
        let opacity = match self {
            Self::Managed(managed) => {
                let inner = managed.state.inner.borrow();
                if inner.owner != Some(owner) {
                    return None;
                }
                inner.visibility.opacity_at(now, managed.config)
            }
            Self::AlwaysVisible => 1.0,
            Self::ManualOpacity(opacity) => *opacity,
        }
        .clamp(0.0, 1.0);
        (opacity > 0.0).then_some(opacity)
    }

    pub(crate) fn current_frame_key(
        &self,
        owner: ScrollbarOwnerKey,
        has_overflow: bool,
        now: Instant,
    ) -> Option<ScrollbarVisibilityKey> {
        if !has_overflow {
            return None;
        }
        let Self::Managed(managed) = self else {
            return None;
        };
        let key = managed.state.visibility_key()?;
        if key.owner == owner
            && managed
                .state
                .inner
                .borrow()
                .visibility
                .is_animating_at(now, managed.config)
        {
            Some(key)
        } else {
            None
        }
    }

    /// Requests one presentation frame only when the full visibility key is current.
    ///
    /// Returns false for obsolete owner, mount, or sequence callbacks and for
    /// policies that are no longer actively interpolating.
    pub fn request_animation_frame_for_key(
        &self,
        expected: ScrollbarVisibilityKey,
        window: &mut Window,
    ) -> bool {
        let Self::Managed(managed) = self else {
            return false;
        };
        if managed.state.visibility_is_current(expected)
            && managed
                .state
                .inner
                .borrow()
                .visibility
                .is_animating_at(Instant::now(), managed.config)
        {
            window.request_animation_frame();
            true
        } else {
            false
        }
    }

    /// Records viewport-originated activity for an exact mounted owner.
    pub fn record_viewport_activity(
        &self,
        owner: ScrollbarOwnerKey,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Self::Managed(managed) = self {
            managed.record_activity(owner, window, cx);
        }
    }

    /// Returns whether an exact current visibility key needs another frame.
    #[must_use]
    pub fn should_request_animation_frame_for_key_at(
        &self,
        expected: ScrollbarVisibilityKey,
        has_overflow: bool,
        now: Instant,
    ) -> bool {
        if !has_overflow {
            return false;
        }
        let Self::Managed(managed) = self else {
            return false;
        };
        managed.state.visibility_is_current(expected)
            && managed
                .state
                .inner
                .borrow()
                .visibility
                .is_animating_at(now, managed.config)
    }

    pub(crate) fn record_direct_activity(
        &self,
        owner: ScrollbarOwnerKey,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Self::Managed(managed) = self {
            managed.record_activity(owner, window, cx);
        }
    }

    pub(crate) fn begin_direct_interaction(
        &self,
        owner: ScrollbarOwnerKey,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Self::Managed(managed) = self {
            managed.state.begin_direct_interaction_with_config(
                owner,
                managed.config,
                window,
                cx,
                managed.on_update.clone(),
            );
        }
    }

    pub(crate) fn end_direct_interaction(
        &self,
        owner: ScrollbarOwnerKey,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Self::Managed(managed) = self {
            managed.state.end_direct_interaction_with_config(
                owner,
                managed.config,
                window,
                cx,
                managed.on_update.clone(),
            );
        }
    }
}

impl ManagedScrollbarVisibility {
    /// Returns the retained keyed state for this managed policy.
    #[must_use]
    pub fn state(&self) -> ScrollbarState {
        self.state.clone()
    }

    fn record_activity(&self, owner: ScrollbarOwnerKey, window: &mut Window, cx: &mut App) {
        self.state.record_activity_with_config(
            owner,
            self.config,
            window,
            cx,
            self.on_update.clone(),
        );
    }
}

/// Explicit visibility policy used by rendered scrollbar chrome.
#[derive(Clone)]
pub enum ScrollbarVisibilityPolicy {
    /// Use keyed managed auto-fading visibility state.
    Managed(ManagedScrollbarVisibility),
    /// Keep scrollbar chrome fully opaque whenever geometry overflows.
    AlwaysVisible,
    /// Use an explicit caller-provided opacity.
    ManualOpacity(f32),
}

/// Cloneable managed visibility policy bound to one retained scrollbar state.
#[derive(Clone)]
pub struct ManagedScrollbarVisibility {
    state: ScrollbarState,
    config: ScrollbarFadeConfig,
    on_update: ScrollbarVisibilityUpdateCallback,
}

impl ScrollbarState {
    /// Builds managed visibility with default timing.
    #[must_use]
    pub fn managed(
        &self,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) -> ScrollbarVisibilityPolicy {
        ScrollbarVisibilityPolicy::managed(self.clone(), on_update)
    }

    /// Builds managed visibility with explicit timing.
    #[must_use]
    pub fn managed_with_config(
        &self,
        config: ScrollbarFadeConfig,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) -> ScrollbarVisibilityPolicy {
        ScrollbarVisibilityPolicy::managed_with_config(self.clone(), config, on_update)
    }

    /// Returns the exact current managed-visibility key.
    #[must_use]
    pub fn visibility_key(&self) -> Option<ScrollbarVisibilityKey> {
        let inner = self.inner.borrow();
        Some(inner.visibility.key(inner.owner?))
    }

    /// Returns managed opacity for the exact owner at a specific instant.
    #[must_use]
    pub fn opacity_at(&self, owner: ScrollbarOwnerKey, now: Instant) -> Option<f32> {
        let inner = self.inner.borrow();
        (inner.owner == Some(owner)).then(|| {
            inner
                .visibility
                .opacity_at(now, ScrollbarFadeConfig::default())
        })
    }

    /// Records managed activity at a specific instant without scheduling UI work.
    pub fn record_activity_at(
        &self,
        owner: ScrollbarOwnerKey,
        now: Instant,
    ) -> Option<ScrollbarVisibilityKey> {
        self.record_activity_at_with_config(owner, now, ScrollbarFadeConfig::default())
    }

    fn record_activity_at_with_config(
        &self,
        owner: ScrollbarOwnerKey,
        now: Instant,
        config: ScrollbarFadeConfig,
    ) -> Option<ScrollbarVisibilityKey> {
        let mut inner = self.inner.borrow_mut();
        (inner.owner == Some(owner))
            .then(|| inner.visibility.record_activity_at(owner, now, config))
    }

    /// Marks keyed direct interaction start at a specific instant.
    pub fn begin_direct_interaction_at(
        &self,
        owner: ScrollbarOwnerKey,
        now: Instant,
    ) -> Option<ScrollbarVisibilityKey> {
        let mut inner = self.inner.borrow_mut();
        if inner.owner != Some(owner) || inner.visibility.direct_interaction {
            return None;
        }
        inner.visibility.direct_interaction = true;
        Some(
            inner
                .visibility
                .record_activity_at(owner, now, ScrollbarFadeConfig::default()),
        )
    }

    /// Marks keyed direct interaction end at a specific instant.
    pub fn end_direct_interaction_at(
        &self,
        owner: ScrollbarOwnerKey,
        now: Instant,
    ) -> Option<ScrollbarVisibilityKey> {
        let mut inner = self.inner.borrow_mut();
        if inner.owner != Some(owner) || !inner.visibility.direct_interaction {
            return None;
        }
        inner.visibility.direct_interaction = false;
        inner.visibility.sequence = inner.visibility.sequence.next();
        inner.visibility.last_activity_at = Some(now);
        inner.visibility.transition = None;
        inner.visibility.animation_task = None;
        Some(inner.visibility.key(owner))
    }

    /// Advances a delay or fade only while its full key remains current.
    pub fn advance_animation_at(
        &self,
        expected: ScrollbarVisibilityKey,
        now: Instant,
        config: ScrollbarFadeConfig,
    ) -> bool {
        let mut inner = self.inner.borrow_mut();
        if inner.owner != Some(expected.owner) || inner.visibility.sequence != expected.sequence {
            return false;
        }
        inner.visibility.advance_animation_at(now, config)
    }

    pub(crate) fn record_activity_with_config(
        &self,
        owner: ScrollbarOwnerKey,
        config: ScrollbarFadeConfig,
        window: &mut Window,
        cx: &mut App,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) {
        let Some(key) = self.record_activity_at_with_config(owner, Instant::now(), config) else {
            return;
        };
        defer_update(self.clone(), key, window, cx, on_update.clone());
        window.refresh();
        self.schedule_animation(config, key, window, cx, on_update);
    }

    pub(crate) fn begin_direct_interaction_with_config(
        &self,
        owner: ScrollbarOwnerKey,
        config: ScrollbarFadeConfig,
        window: &mut Window,
        cx: &mut App,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.owner != Some(owner) || inner.visibility.direct_interaction {
                return;
            }
            inner.visibility.direct_interaction = true;
        }
        self.record_activity_with_config(owner, config, window, cx, on_update);
    }

    pub(crate) fn end_direct_interaction_with_config(
        &self,
        owner: ScrollbarOwnerKey,
        config: ScrollbarFadeConfig,
        window: &mut Window,
        cx: &mut App,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) {
        let Some(key) = self.end_direct_interaction_at(owner, Instant::now()) else {
            return;
        };
        defer_update(self.clone(), key, window, cx, on_update.clone());
        window.refresh();
        self.schedule_animation(config, key, window, cx, on_update);
    }

    fn schedule_animation(
        &self,
        config: ScrollbarFadeConfig,
        expected: ScrollbarVisibilityKey,
        window: &mut Window,
        cx: &mut App,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) {
        let Some(delay) = self.next_animation_delay_at(Instant::now(), config, expected) else {
            return;
        };
        let state = self.clone();
        let task = window.spawn(cx, async move |cx| {
            cx.background_executor().timer(delay).await;
            let _ = cx.update(move |window, cx| {
                state.advance_animation(config, expected, window, cx, on_update);
            });
        });
        let mut inner = self.inner.borrow_mut();
        if inner.owner == Some(expected.owner) && inner.visibility.sequence == expected.sequence {
            inner.visibility.animation_task = Some(task);
        }
    }

    fn next_animation_delay_at(
        &self,
        now: Instant,
        config: ScrollbarFadeConfig,
        expected: ScrollbarVisibilityKey,
    ) -> Option<Duration> {
        let inner = self.inner.borrow();
        if inner.owner != Some(expected.owner) || inner.visibility.sequence != expected.sequence {
            return None;
        }
        inner.visibility.next_animation_delay_at(now, config)
    }

    fn advance_animation(
        &self,
        config: ScrollbarFadeConfig,
        expected: ScrollbarVisibilityKey,
        window: &mut Window,
        cx: &mut App,
        on_update: ScrollbarVisibilityUpdateCallback,
    ) {
        let should_update = {
            let mut inner = self.inner.borrow_mut();
            if inner.owner != Some(expected.owner) || inner.visibility.sequence != expected.sequence
            {
                return;
            }
            inner.visibility.animation_task = None;
            inner
                .visibility
                .advance_animation_at(Instant::now(), config)
        };
        if should_update {
            defer_update(self.clone(), expected, window, cx, on_update.clone());
            window.refresh();
        }
        if self.visibility_key() == Some(expected) {
            self.schedule_animation(config, expected, window, cx, on_update);
        }
    }

    pub(crate) fn visibility_is_current(&self, expected: ScrollbarVisibilityKey) -> bool {
        self.visibility_key() == Some(expected)
    }
}

fn defer_update(
    state: ScrollbarState,
    expected: ScrollbarVisibilityKey,
    window: &mut Window,
    cx: &mut App,
    on_update: ScrollbarVisibilityUpdateCallback,
) {
    window.defer(cx, move |window, cx| {
        if state.visibility_is_current(expected) {
            on_update(expected, window, cx);
        }
    });
}
