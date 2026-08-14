# Scope

The keyed GPUI scrollbar pointer-release lifecycle.

# Invalidated Approach

Address active custom-renderer dragging by owner identity or by whichever drag is current.

# Evidence

GPUI dispatches mouse-up through the latest render closure after a normal redraw, even though the
active drag began from an earlier press. Separately, completion review showed that public
`start_drag -> bool`, unkeyed update/completion, and owner-keyed cancellation allowed late
instance-A events to affect recurrent same-owner instance B.

The receipt correction then exposed the same erasure after a caller update callback: that callback
can settle A and start B reentrantly, so an owner-only snapshot writeback on A's return could modify
B.

A receipt that contained only a per-state instance counter could also match instance one in a
different retained scrollbar state. Equal geometry does not make those state allocations one drag.

A semantic rerender could replace the pending press constructor without preserving the old exact
pending identity for latest-render mouse-up, leaving the old pending state stranded and blocking a
fresh press. A start callback can mutate geometry or reenter lifecycle after `drag_started`, while
completion snapshot lookup can reenter retained state; neither path may rely on owner/current state
or retain a `RefCell` borrow.

# Course Correction

Mounted handlers retain the exact pending or active instance needed by GPUI dispatch. Semantic
rerender makes the retained pending press unpromotable while giving the latest release handler that
same instance, so only that press settles. Public custom renderers receive an opaque crate-issued
receipt after their start callback succeeds and must present it for update, completion, or
cancellation; the receipt is still returned if callback reentrancy already settled the exact drag.
A mismatched receipt is rejected before another active drag is inspected or mutated. Every
post-callback writeback rechecks the exact active instance before it changes retained state. Each
receipt additionally carries a private `Weak` reference to the issuing state allocation, so
receipt-consuming operations reject a foreign or dropped origin without retaining that state.
Lifecycle state is copied out before an external snapshot provider or callback runs, then exact
owner, pending, and active identities are revalidated before mutation.

Provider reentrancy can cancel A and start recurrent B between a snapshot result and a later
`take()`, or between a drag update check and owner callback. Completion must therefore inspect the
active slot conditionally after every provider return, never take before exact matching, use a
private settling barrier for A's one authorized end callback, and remove only unchanged A after
that callback. Update and lane dispatch likewise revalidate immediately before their scroll or page
callbacks and after callbacks before retained writeback.

The same ABA rule applies before promotion: removing pending A before its snapshot provider lets a
reentrant call occupy and settle B, then lets obsolete A install after recurrent geometry returns.
Pending A now remains in its slot as an exact private `Promoting` entry across the provider call.
Only that unchanged owner/render/instance/snapshot/style/interaction entry can transition to active;
reentrant starts are rejected, while exact cancellation or owner replacement removes A and makes the
outer promotion reject without touching any later work.

# Remaining Risk

Mounted tests and the custom-renderer A-late-versus-B proof must remain when GPUI drag integration
changes.
