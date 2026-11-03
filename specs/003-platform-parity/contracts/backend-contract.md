# Backend Contract

**Feature**: `003-platform-parity`
**Date**: 2026-11-03

What any platform backend must guarantee to be considered conforming. This contract is internal — it does not extend the public `rn` API, which is unchanged by this feature.

---

## C1. The trait surface

A backend implements the existing two-method trait and nothing more is required of it by the framework:

```rust
pub trait PlatformRenderer {
    fn apply(&mut self, changeset: Changeset) -> Result<(), RenderError>;
    fn run_event_loop(&mut self) -> Result<(), RenderError>;
}
```

Each backend additionally exposes a constructor `new() -> Result<Self, RenderError>` and is re-exported as `rn::backend::{WindowsRenderer, LinuxRenderer}` behind its Cargo feature.

**Unchanged**: this feature adds no method to the trait. If a backend appears to need one, that is a signal the changeset is under-specified and the fix belongs in `rn-core`, not in the contract.

---

## C2. Changeset obligations

`apply` receives `Operation::Create`, `Update`, `Delete`, and `Reorder`.

| Obligation | Failure mode if unmet |
|---|---|
| **Create** must produce a live native control for every node and store it in the control record before returning | Orphan nodes; invisible UI |
| **Update** must apply only the changed properties, using the property-to-native-call table in [data-model.md](../data-model.md#cross-platform-state-coverage) | State desync: a toggle that looks off but reports on |
| **Delete** must destroy the native control and clear the record slot | Handle leak; growth without bound |
| **Reorder** must reposition existing children without destroy-and-recreate | Flicker; lost focus and scroll position |

The backend MUST NOT re-walk the component tree. Everything it needs is in the changeset (constitution VI).

---

## C3. Geometry

- Position and size come from the shared layout result, in logical units, top-left origin.
- The backend MUST NOT compute, adjust, or infer geometry.
- The backend MUST flip the Y axis if its platform uses a bottom-left origin; this is the one permitted transformation.
- Scaling to device pixels is a backend concern and MUST NOT feed back into layout (research R6).

---

## C4. Completeness

- The `ComponentKind` match in `apply`'s create and update paths MUST be exhaustive with no `_` catch-all arm.
- `capabilities()` MUST enumerate every kind the backend truly renders.
- A coverage test MUST assert `capabilities().len() == ComponentKind::COUNT`.

**Adding a component kind later is therefore a compile error on every backend that has not implemented it.** That is the intended behaviour, not an inconvenience.

---

## C5. Events

Every backend emits the same payload shapes for the same user action:

| Interaction | Event |
|---|---|
| Button, list tile, sidebar item activated | `OnPress` |
| Toggle, switch, checkbox changed | `OnChange` with the new boolean |
| Dropdown, segmented control, radio selected | `OnChange` with the new index |
| Text edited | `OnChange` with the new string |
| Text submitted (Enter) | `OnSubmit` |
| Scroll | `OnScroll` with offset and extent |
| Window close requested | `OnClose` |

Event bubbling for `TouchableOpacity` and other nested interactive containers MUST behave identically to macOS: walk the shadow-tree parent chain, fire each ancestor's `OnPress`, stop when `stop_bubbling` is set.

---

## C6. Failure behaviour

- No `unwrap`, `expect`, or `panic` on any path reachable from `apply` or the event loop.
- A recoverable failure returns `Err(RenderError::…)` and leaves prior UI intact; it MUST NOT abort the process.
- A missing native control for a live node yields a diagnostic naming the node and kind, not a silently absent control.

---

## C7. Resources

- No allocation inside `apply` or event dispatch, except growth of the control vector when a node id exceeds capacity (research R3).
- Every native control created is destroyed exactly once.
- The event loop returns cleanly when the last window closes.

---

## C8. Platform obligations

Beyond the shared contract:

**Windows**
- Declare per-monitor DPI awareness v2 in the application manifest.
- Handle `WM_DPICHANGED` by re-applying the last layout at the new scale factor.
- Pump messages on the thread that created the window.

**Linux**
- Run under both Wayland and X11; do not assume either.
- Follow the desktop's light/dark preference and update live when it changes.
- Run the main loop through `GtkApplication`, not a manual loop.

---

## C9. What this contract deliberately does not require

- **Visual identity with macOS.** A Windows backend that looks like macOS is a failed backend (FR-016).
- **Animation.** Out of scope for this feature.
- **Custom-drawn fallbacks.** If a native control exists, it is used. If none exists, the kind is composed from native primitives and that composition is recorded in the data model.
