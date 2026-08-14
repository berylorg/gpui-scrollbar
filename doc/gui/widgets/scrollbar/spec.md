# Name

Canonical name: scrollbar

Sometimes known as: scroll thumb, scroll lane

# Purpose

`scrollbar` is an app-neutral GPUI scrollbar primitive for visible overflow position, managed thumb visibility, lane hit testing, thumb dragging, and lane page requests.

The widget does not own scroll state, focus routing, keyboard input, wheel routing, touchpad routing, virtualization, persistence, or app-specific edge behavior.

# References

Contracts: N/A

Widgets: N/A

# Anatomy

The widget contains an interactive hit lane, a computed track geometry, a visible rounded thumb, and an optional zero-size animation-frame driver used while managed opacity transitions are active.

The hit lane owns the element id and pointer handlers. The track is not painted by the default renderer. The thumb is the only visible element in the default renderer.

# Look

The default visible thumb reads as rounded and visually quiet. The hit lane is larger than the thumb so direct manipulation remains usable without painting a separate track.

Opacity is supplied by `ScrollbarVisibilityPolicy`. The default renderer has no text, icons, focus ring, hover color, or painted track background.

# States

Supported states are no-overflow hidden, visible overflow, managed hidden, managed fading in, managed visible, managed fading out, direct interaction, dragging, drag released, drag cancelled, fade pending, fade cancelled, fade obsolete, always visible, manual opacity, vertical, horizontal, thumb hit, lane-before-thumb hit, and lane-after-thumb hit.

The widget is not rendered when there is no positive overflow, no positive viewport length, no positive track length, or an opacity policy resolves to zero opacity.

# Interaction

Left mouse down on the lane stops event propagation.

Pressing the thumb starts one drag bound to the caller-supplied owner identity, current mount
generation, and exact semantic render epoch that installed the drag constructor, with the pointer's
grab offset preserved. An unchanged GPUI press redraw retains that epoch. Any intervening rendered
snapshot, normalized style, or interaction instance advances it, so geometry returning to an older
value cannot promote an obsolete press. A semantic rerender carries that retained press's exact
pending identity into the latest release handler, but it remains unpromotable and cannot affect a
later fresh press. Dragging maps pointer movement back to a caller-owned scroll offset and invokes
the owner update callback only while that drag key remains current.

Each pending and active drag value has an exact private instance identity. Redraw may discard an
unused replacement value without cancelling the genuine active value retained by GPUI. A recreated
scroll-handle adapter remains the same interaction only for a clone of the same actual
`ScrollHandle`; a separately constructed equal-state handle invalidates the retained press.

Custom interactive renderers receive an opaque receipt only after a drag starts. The receipt is
privately bound, without retaining it, to the issuing scrollbar state. They present that same
receipt for move, release, or cancellation; a stale or foreign-state receipt cannot affect a
recurrent drag under the same owner and geometry.

After the start callback is invoked, custom start returns that exact receipt even when callback
reentrancy has already settled its drag. Snapshot providers and callbacks are invoked without a
retained-state borrow and exact identity is checked again before lifecycle state changes.

Every provider result is rechecked before a scroll, page, or lifecycle callback. Completion uses an
already-authorized exact snapshot for its end callback, marks only that drag as settling, and removes
only that unchanged instance after the callback; a reentrant replacement drag is never taken by old
cleanup.

Pointer release completes the current drag exactly once, invokes the drag-end callback, releases
pointer capture and retained drag state, and records managed-visibility activity. Pointer-capture
cancellation, owner replacement, viewport or scrollbar unmount, and window teardown cancel the
drag exactly once, release the same state, and do not synthesize a final scroll update. Pointer
movement or release delivered after completion or cancellation is ignored for the obsolete drag
key.

A vertical lane click outside the thumb uses the current owner key and current hit-tested viewport,
track, and thumb geometry. It requests the before-thumb or after-thumb direction plus a positive
page distance supplied by the owner for that same geometry snapshot, normally the current viewport
length; the owner applies its own edge clamping. A geometry or owner-key mismatch rejects the click
instead of dispatching a page from a stale thumb position. Horizontal lane clicks outside the thumb
do nothing and dispatch no page request.

Each managed inactivity delay and fade is keyed by owner identity, mount generation, and a
monotonic visibility sequence. New activity supersedes the prior sequence. Owner replacement,
viewport or scrollbar unmount, and window teardown cancel pending delay or fade work and release
its timer and animation-frame driver. A timer wakeup or animation-frame callback must match the
current key and sequence immediately before mutation; a late completion is obsolete and cannot
change opacity, request another frame, invoke an owner callback, or keep the driver alive.

The widget does not implement keyboard scrolling, wheel scrolling, focus traversal, or touch gestures.

# Layout

The vertical variant is absolutely positioned at the top and right of the viewport, spans full viewport height, and uses `hit_lane_thickness` as its inline size. Its thumb is positioned from the right edge by `track_inset`, from the top edge by `track_inset + thumb_offset`, uses `thickness` as width, and uses computed `thumb_length` as height.

The horizontal variant is absolutely positioned at the left and bottom of the viewport, spans full viewport width, and uses `hit_lane_thickness` as its block size. Its thumb is positioned from the bottom edge by `track_inset`, from the left edge by `track_inset + thumb_offset`, uses computed `thumb_length` as width, and uses `thickness` as height.

Thumb length is proportional to viewport length over content length, clamped by minimum thumb length and available track length. Thumb offset is proportional to clamped scroll offset over overflow length.

One geometry snapshot contains the owner identity, mount generation, axis, viewport length, content
length, clamped scroll offset, track bounds, thumb bounds, and page distance used for one pointer
hit. Page direction and distance are derived from that same current snapshot so a lane request
cannot combine a newly measured viewport with an obsolete thumb.

# Variants

Default variant: interactive managed scrollbar.

Supported variants are vertical, horizontal, scroll-handle backed, callback backed, managed visibility, always visible, manual opacity, full interactive scrollbar, and non-interactive thumb-only rendering.

# UI Roles

```css
.scrollbar {
  --hit-lane-thickness: 18px;
  --track-inset: 6px;
  --thumb-thickness: 4px;
  --thumb-min-length: 24px;
}

.scrollbar__thumb {
  --background: #94a3b8;
  --opacity: 1;
  --radius: 999px;
}
```
