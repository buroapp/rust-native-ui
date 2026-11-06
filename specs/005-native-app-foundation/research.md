# Research: Native App Foundation

**Feature**: `005-native-app-foundation` | **Phase**: 0 | **Date**: 2026-11-05

Unknowns extracted from the plan's Technical Context, resolved here. Technical Context
carries no remaining `NEEDS CLARIFICATION` markers after these decisions.

---

## R1. How to render 100,000 rows without breaking the render budget

**Decision**: Rows are data owned by the component, never component nodes. The
component receives a model and hands it to the platform's table data source, which asks
for cells only for rows in the visible rectangle.

**Rationale**: Article IV allows 8ms for layout and dispatch. A 100,000-node subtree
through taffy fails that by orders of magnitude before a single cell is drawn, and a
component-per-row design also allocates on every data change. Native table views are
already virtualized, so the correct move is to not duplicate that work in our tree.

**Alternatives considered**:
- *Rows as component nodes with reconciler-level virtualization.* Rejected: requires
  partial materialization inside the core reconciler loop, a far larger change than the
  documented carve-out, and still allocates per row on every update.
- *`FlatList`-style recycling engine extended with columns.* Rejected: reimplements
  column resizing, selection, sorting and keyboard navigation that the native control
  already provides, and would be a custom-drawn table, prohibited by Article I.

**Consequence**: `ComponentKind::Table` and `ComponentKind::Tree` declare a fixed
intrinsic height and are a single layout node each, exactly as `FlatList` is today.

---

## R2. How to get responder-chain-correct shortcut dispatch

**Decision**: Declare commands as data and build the platform's main menu with key
equivalents pointing at the framework's existing handler dispatch. Rely on the platform's
own key-equivalent routing, which offers the key to the first responder before the menu.

**Rationale**: FR-005 requires that a focused text input which handles a key prevents an
application command from stealing it. The platform's event path already implements
exactly that ordering. Reimplementing it as a global key hook would both fight the
platform and break the moment focus moves into a native control we do not own.

**Alternatives considered**:
- *Global keyboard hook with our own precedence table.* Rejected: duplicates platform
  behaviour, must special-case every native control, and cannot see first-responder state
  reliably.
- *Per-widget shortcut registration.* Rejected: shortcuts are application-scoped; per
  widget registration makes conflicts undetectable and menu state impossible to compute
  (FR-004).

**Open detail for implementation**: main menu installation is application-wide state.
The plan records three candidate policies (first window installs, later windows merge, or
commands declared once per application). This is a real decision, not an oversight, and
belongs in the implementation of US1.

---

## R3. How to anchor overlays

**Decision**: Use the platform's popover for popovers, the platform's tooltip mechanism
for hover text, and the platform's menu presentation for anchored menus. Placement is
handled by the native control with our anchor frame supplied.

**Rationale**: FR-020 requires the overlay to stay inside the window, and FR-021 requires
it to track or dismiss as the anchor moves. Native popovers implement edge avoidance,
repositioning and dismissal as part of their contract. Our current popover is a
placeholder label precisely because we tried to position it ourselves.

**Alternatives considered**:
- *Custom positioned panels.* Rejected: reimplements edge avoidance and dismissal, and
  produces an overlay that is not a native overlay, against Article I.

---

## R4. Which native control for the editor

**Decision**: The platform's scrollable text view, wrapped by the framework, exposing
text content in and out through the existing prop mechanism.

**Rationale**: FR-025 and FR-026 require caret, selection, insert, replace, delete, copy,
paste, undo and redo. The native text view has all of it, including an undo manager that
cooperates with the responder chain. Building any of it by hand is both larger and worse.

**Alternatives considered**:
- *Extending the existing rich-text display control.* Rejected: a display control has no
  editing stack, no undo, no selection model.
- *Single-line input.* Rejected by FR-025 (multi-line).

---

## R5. Reduced motion detection

**Decision**: Read the platform's accessibility preference for reduce motion at
transition time, and set the transition duration to zero when it is on.

**Rationale**: FR-032 requires instantaneous change under that preference with an
identical end state. The platform exposes the preference as a queryable value, so no
polling or notification plumbing is required for correctness; querying at transition time
also satisfies the "toggled while running" edge case without a restart.

**Alternatives considered**:
- *Cache the preference at launch.* Rejected: fails the edge case where the user changes
  it while the application is open.
- *Subscribe to preference-change notifications.* Deferred: correct but unnecessary until
  a screen needs to react before the next transition.

---

## R6. Headless testing approach

**Decision**: A test session that drives the same reconciler and layout path as
production, with input dispatched into the same handler path, and an offscreen window
used only when native accessibility assertions are required.

**Rationale**: FR-033 and FR-034 require asserting on state, computed layout and
accessibility without a visible window. Testing a separate simulation would let real
layout bugs pass, so the session must reuse the production path. Accessibility output
cannot be produced without a native view hierarchy, so the offscreen window is used for
those assertions only.

**Alternatives considered**:
- *Pure model-level tests.* Already exist and are insufficient: they cannot catch layout
  or accessibility regressions, which is the point of US8.
- *Always using a window and hiding it.* Rejected: hidden windows still take part in some
  platform machinery and slow the suite; the offscreen path stays off by default.

---

## R7. Keeping the public API free of platform types

**Decision**: Data models live in `rn-core` as plain Rust structs and enums. The macOS
crate converts them into native data sources. No `objc2` type appears in any signature
reachable from `rn`.

**Rationale**: Article III forbids platform types leaking into the public API. The existing
`ToolbarActions` payload shows the pattern already works: structured data crosses the
boundary, the platform crate owns the native objects.

**Alternatives considered**:
- *Trait objects supplied by the application to act as data sources.* Rejected: it would
  drag the platform's pull-model lifetime and threading requirements into the public API,
  which is exactly the leak Article III prohibits, and it makes the model un-testable
  without a platform.

---

## R8. Where the new code lives

**Decision**: New pure models in `rn-core/src/data/`. New native implementations as
separate modules in `rn-macos` (`table.rs`, `tree.rs`, `commands.rs`, `overlays.rs`,
`motion.rs`) rather than more arms in `renderer.rs`.

**Rationale**: `renderer.rs` is already about 3,400 lines. Three of the five areas add a
data source or a delegate, which is a class with lifetimes and callbacks, not a match arm.
Splitting keeps the match arms thin and keeps the data source code reviewable on its own.

**Alternatives considered**:
- *Everything in `renderer.rs`.* Rejected: it is already the hardest file in the project
  to navigate, and this feature would roughly double it.

---

## Dependencies and integration points confirmed against the repository

- `objc2-app-kit` already enables `NSOutlineView`, `NSTableView`, `NSTableColumn`,
  `NSTableCellView`, `NSTableRowView`, `NSTableHeaderView`, `NSUserInterfaceItemIdentification`,
  `NSSplitViewController`, `NSViewController`. The table and tree work needs no new
  dependency, only new arms against features already present.
- `PropValue` already carries structured payloads (`ToolbarActions`, `Spans`, `NodeRef`),
  so adding data-model variants is an established pattern rather than a new one.
- `FlatListState` in `rn/src/flat_list.rs` already proves a data-driven component can live
  in this architecture without entering the reconciler tree.
- The sidebar's `NSOutlineView` implementation in `rn-macos/src/renderer.rs` already
  demonstrates the pull-model data source and delegate pattern, including selection
  dispatch into the existing handler path. This is the de-risking precedent for US3.
- `ComponentKind::ALL`, `ComponentKind::COUNT`, the storybook catalogue drift test and the
  two stub backends' exhaustive matches all enforce completeness. Every new kind touches
  all four.
