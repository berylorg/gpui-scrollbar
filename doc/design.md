# Goals

Provide app-neutral scrollbar primitives for native `gpui` applications.

The crate exists so reusable `gpui` projects can share scrollbar geometry, rendering, and pointer direct-manipulation behavior without depending on an application shell.

## Non-goals

- Owning application scroll state, content virtualization, focus routing, pointer-wheel routing, keyboard shortcuts, persistence, or application data semantics.
- Owning a product-specific theme schema.
- Owning transcript, settings-window, workspace, or other application-domain behavior.
- Replacing or patching `gpui` scroll and list primitives.

# Decisions

## Crate Boundary

- The crate owns app-neutral scrollbar geometry, thumb rendering, thumb dragging, and vertical scrollbar-lane page-click dispatch for `gpui` views.
- The crate exposes reusable primitives for ordinary `gpui` scroll handles and for caller-owned scroll models.
- The crate depends on `gpui` and app-neutral Rust dependencies only.
- The public API must not expose Beryl shell, transcript, workspace, settings, or product-theme types.
- Callers supply scrollbar identity, current scroll geometry, styling, opacity, and callbacks into their own scroll model.
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
- The rendered scrollbar is thumb-first chrome; callers decide when the scrollbar is visible and what opacity to pass for fade effects.
- The crate does not own application-level hover, activity, or fade scheduling policy.

## Interaction Lifecycle

- Thumb dragging preserves the pointer's grab offset within the thumb until pointer release or cancellation.
- Drag start, drag update, drag end, and lane-click activity are surfaced through caller-provided callbacks where the caller needs to update activity or scroll intent state.
- Callback-based scrolling is the only path for mutating scroll position, so caller-owned viewport semantics remain authoritative.
