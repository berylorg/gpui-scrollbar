# Goals

Provide app-neutral scrollbar primitives for native `gpui` applications.

The crate exists so reusable `gpui` projects can share scrollbar geometry, rendering, managed visibility behavior, and pointer direct-manipulation behavior without depending on an application shell.

## Non-goals

- Owning application scroll state, content virtualization, focus routing, pointer-wheel routing, keyboard shortcuts, persistence, or application data semantics.
- Owning a product-specific theme schema.
- Owning transcript, settings-window, workspace, or other application-domain behavior.
- Replacing or patching `gpui` scroll and list primitives.

# Decisions

## Crate Boundary

- The crate owns app-neutral scrollbar geometry, thumb rendering, managed visibility and fade behavior, thumb dragging, and vertical scrollbar-lane page-click dispatch for `gpui` views.
- The crate exposes reusable primitives for ordinary `gpui` scroll handles and for caller-owned scroll models.
- The crate depends on `gpui` and app-neutral Rust dependencies only.
- The public API must not expose Beryl shell, transcript, workspace, settings, or product-theme types.
- Callers supply scrollbar identity, current scroll geometry, styling, retained scrollbar state, and callbacks into their own scroll model.
- Ordinary rendering APIs require an explicit visibility policy. Auto-fading visibility uses crate-owned state stored by the caller, while non-fading visibility is an explicit API choice rather than an accidental default.
- Callback-based scrolling uses positive visible scroll distances along the scrollbar axis; helpers that wrap `gpui::ScrollHandle` adapt GPUI's content-offset convention at the crate boundary.

## Scroll Ownership

- A scrollable viewport owns its scroll state, scroll bounds, page size, content measurement, and edge behavior.
- A scrollable viewport or its integrating surface owns keyboard scrolling commands such as line scroll, page scroll, and home/end.
- A scrollable viewport or its integrating surface owns pointer-wheel and touchpad routing, including nested-scrollable selection rules.
- The scrollbar owns only pointer interactions that originate on scrollbar chrome.
- Thumb dragging maps pointer movement into caller-owned scroll-position callbacks without directly owning the caller's scroll state.
- Clicking a vertical scrollbar lane outside the thumb dispatches one caller-owned page-scroll callback with direction and page distance.
- Horizontal lane clicks outside the thumb do not dispatch page scrolling unless a later explicit API adds that behavior.
- The crate must not register keyboard bindings, decide which viewport is focused, or implement application-specific edge rules.

## Styling

- The crate provides an app-neutral style type for dimensions, hit-lane sizing, thumb color, and related visual constants.
- Callers may supply style values derived from their own theme systems.
- The rendered scrollbar is thumb-first chrome; its full outline or track remains a hit lane rather than required painted chrome.
- The crate produces managed scrollbar opacity from its visibility policy instead of requiring ordinary callers to compute fade opacity.

## Visibility Lifecycle

- Managed auto visibility shows scrollbar chrome only when the current scroll geometry overflows and recent scrollbar activity makes the affordance relevant.
- Pointer activity over an owning scrollable region, pointer-wheel or touchpad scrolling routed to that region, thumb dragging, and lane clicking are app-neutral activity signals for managed visibility.
- Direct scrollbar interactions record managed visibility activity as part of the scrollbar chrome lifecycle.
- Activity that originates outside scrollbar chrome is reported by the owning viewport or integrating surface; the crate does not install wheel handlers, keyboard handlers, focus routing, or nested-scroll routing.
- The managed visibility state is per scroll region so independent scrollable surfaces do not reveal or fade each other's scrollbar chrome.
- Managed visibility repaint callbacks are deferred through the `gpui` effect cycle, so a viewport may report activity from its own event handler without re-entering its owner entity.
- Managed fade interpolation is driven from the scrollbar render path by requesting `gpui` animation frames while a fade transition is active, so visible opacity changes follow the window presentation cadence. Timer wakeups may start or finish lifecycle phases, but they must not be the only mechanism advancing visible fade frames.
- Always-visible scrollbar chrome is available only through an explicit visibility policy for surfaces that intentionally need it.

## Interaction Lifecycle

- Thumb dragging preserves the pointer's grab offset within the thumb until pointer release or cancellation.
- A pending thumb press is bound to the exact semantic render epoch that installed its drag constructor, in addition to owner, mount, geometry, normalized style, and interaction instance. An unchanged GPUI press redraw retains that epoch, while any intervening constructor state advances it so later geometry reversion cannot revive the obsolete press.
- A semantic rerender makes a retained pre-threshold press permanently unpromotable, but carries its exact pending instance into the newest release handler so that release can settle that press without affecting any later pending or active drag.
- Pending and active drag values carry private exact instance identities. Ordinary redraw may replace unused drag values without cancelling the active value retained by GPUI; keyed release or cancellation settles the matching pending or active instance, and destruction cancels only the active instance owned by that value.
- Once the public custom-renderer start invokes its start callback, it returns that drag's opaque receipt even if reentrancy has already settled it. The receipt is privately bound to its issuing retained-state allocation through a non-retentive reference; callers may observe only that its issuer has dropped, while its update, completion, and cancellation operations reject a stale or foreign-state receipt before inspecting or mutating another active drag.
- Snapshot providers and lifecycle callbacks run only after retained-state borrows have been released; their reentrant exact-key mutations are revalidated before any subsequent state write.
- Every provider result is revalidated against the exact active key before state mutation or a caller scroll/page/end callback. Completion marks the exact active drag as settling before its authorized end callback, and then removes only that unchanged instance.
- Recreated scroll-handle adapters compare the actual retained handle allocation through GPUI's opaque pointer-equality query. Clones of one handle preserve interaction continuity, while separately constructed equal-state handles invalidate a retained press.
- Drag start, drag update, drag end, and lane-click activity update managed visibility state and are surfaced through caller-provided callbacks where the caller needs to update scroll intent state.
- Callback-based scrolling is the only path for mutating scroll position, so caller-owned viewport semantics remain authoritative.

## Frame-Driver Test Observation

The opt-in `test-support` feature exposes a bounded `FrameDriverProbe` attached to one managed
visibility policy with `with_frame_driver_probe`. Policy clones share its observation. It retains
only driver-call and actual GPUI frame-request counts plus each path's latest exact visibility key;
it does not retain callbacks, windows, scrollbar state, or an event history. Attaching it to a
non-managed policy is a test setup error. The observer neither recomputes admission nor changes
visibility or scheduling behavior and is absent from default builds.

Mounted lifecycle evidence must distinguish ordinary activation redraws from frame requests. The
obsolete-driver case verifies actual driver execution and no admitted request for its retired key;
a current-key control verifies that the same observation records real frame requests.

# Engineering Rigor

Profile: `production-application/v1`

Modifiers: none
