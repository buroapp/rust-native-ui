# Feature Specification: Core Framework

**Feature Branch**: `001-core-framework`

**Created**: 2026-11-02

**Status**: Draft

## User Scenarios & Testing

### User Story 1 — Write Once, Run Native Everywhere (Priority: P1)

A Rust developer describes their UI once using `rn`'s declarative component API.
The same source compiles and renders using native OS controls on macOS, Windows,
and Linux without any platform-specific branches in consumer code.

**Why this priority**: This is the entire value proposition of the framework.
Nothing else matters if this doesn't work.

**Independent Test**: A minimal "Hello World" app with a window, a label, and a
button can be compiled and launched natively on all three platforms. The controls
are verified to be real OS-native widgets (not custom-drawn). Delivers standalone
value as a proof-of-concept.

**Acceptance Scenarios**:

1. **Given** a Rust project depending on `rn`, **When** a developer defines a
   component tree using only the `rn` public API, **Then** the same source file
   compiles on macOS, Windows, and Linux without modification.

2. **Given** a compiled `rn` app running on macOS, **When** the window is
   inspected via Accessibility Inspector, **Then** all controls are identified as
   native AppKit widgets.

3. **Given** a compiled `rn` app running on Windows, **When** the window is
   inspected via Accessibility Insights, **Then** all controls are identified as
   native Win32/WinUI controls.

4. **Given** a compiled `rn` app running on Linux, **When** the window is
   inspected via accerciser, **Then** all controls are identified as native GTK4
   widgets.

---

### User Story 2 — State Change Triggers Minimal Native Update (Priority: P2)

When application state changes, only the native controls affected by that state
change are updated. The full widget tree is not torn down and rebuilt.

**Why this priority**: Performance correctness. A naive full-rebuild approach
would technically satisfy P1 but produce unusable flicker and jank at any
meaningful UI complexity.

**Independent Test**: An app with a counter and a label. Incrementing the counter
updates only the label's text. The window, button, and any sibling controls are
not re-created. Verifiable via platform accessibility event logs showing a single
property-change event, not a control-destroy/create pair.

**Acceptance Scenarios**:

1. **Given** a rendered component tree, **When** a single leaf state value changes,
   **Then** only the native control(s) bound to that value receive an update call.

2. **Given** a component subtree that is removed from state, **When** the reconciler
   runs, **Then** the corresponding native controls are destroyed and no memory leak
   is observable after 1000 such cycles.

3. **Given** no state change, **When** the event loop ticks, **Then** no native
   control update calls are issued.

---

### User Story 3 — Platform Types Never Leak to Consumer (Priority: P3)

A developer using `rn` never imports `objc2`, `windows`, or `gtk4` directly.
All interaction with the framework goes through `rn`'s own types and traits.

**Why this priority**: API hygiene. Platform type leakage forces consumers to
manage per-platform compile-time cfg guards, defeating the purpose of the
abstraction.

**Independent Test**: A consumer crate with `rn` as the only UI dependency
compiles successfully on all three platforms. Adding `deny(unused_extern_crates)`
to the consumer shows no platform-backend crate in scope.

**Acceptance Scenarios**:

1. **Given** a consumer crate depending only on `rn`, **When** it is compiled,
   **Then** no symbols from `objc2`, `windows-rs`, or `gtk4-rs` appear in the
   consumer's public or private namespace.

2. **Given** a consumer handling a button-press event, **When** the event is
   received, **Then** its type is `rn::event::Event` (or equivalent), not a
   platform-specific event type.

---

### Edge Cases

- What happens when a component requests a widget type that has no native
  equivalent on the current platform? (Expected: compile error or explicit
  fallback widget declared by the consumer.)
- How does the framework behave when the OS denies a control creation call?
  (Expected: surfaces as `Result::Err` to the consumer, never panics.)
- What happens when state changes faster than the event loop can process?
  (Expected: updates are coalesced; intermediate states may be skipped but the
  final state is always rendered.)

## Requirements

### Functional Requirements

- **FR-001**: The framework MUST expose a single platform-agnostic API crate (`rn`)
  that consumer code depends on exclusively.
- **FR-002**: The framework MUST provide a declarative component trait or type that
  consumers use to describe their UI as a pure value (no side effects at description
  time).
- **FR-003**: The framework MUST include a reconciler that diffs two consecutive
  component trees and produces a minimal changeset.
- **FR-004**: The framework MUST include at least one platform renderer that consumes
  a changeset and applies it to real native OS controls.
- **FR-005**: Platform renderer backends MUST be selected at compile time via Cargo
  feature flags; no runtime platform detection is used for control dispatch.
- **FR-006**: The reconciler MUST be independently testable without a display server
  or OS (headless unit tests).
- **FR-007**: Consumer code MUST NOT be required to hold or manage native control
  handles directly.
- **FR-008**: All framework fallibility MUST surface as `Result` or `Option`; no
  panics in library code.
- **FR-009**: The framework MUST support at minimum: window, label, button, text
  input, and vertical/horizontal layout container as primitive widget types.
- **FR-010**: The framework MUST deliver a changeset to the platform renderer within
  8ms of a state change on minimum target hardware for a tree of up to 500 nodes.

### Key Entities

- **Component**: A pure value describing a UI element — its type, properties, and
  children. Produces no side effects when constructed.
- **ComponentTree**: An owned, immutable snapshot of the full UI description at a
  point in time.
- **Changeset**: The minimal set of create/update/delete/reorder operations produced
  by the reconciler from diffing two ComponentTrees.
- **PlatformRenderer**: A per-platform implementation that accepts a Changeset and
  applies it to real native OS controls.
- **Event**: A platform-agnostic type representing user input (click, keypress,
  focus, etc.) emitted by the platform renderer and delivered to consumer handlers.

## Success Criteria

### Measurable Outcomes

- **SC-001**: A developer with Rust experience but no prior `rn` knowledge can build
  and run a working three-widget app on their native OS within 30 minutes of reading
  the quickstart.
- **SC-002**: State changes in a 500-node component tree are reconciled and applied
  to native controls within 8ms on minimum target hardware (see constitution).
- **SC-003**: The same consumer source file compiles and produces a runnable native
  app on all three supported platforms without modification.
- **SC-004**: A consumer crate depending only on `rn` has zero direct dependencies
  on any platform-backend crate in its dependency graph.
- **SC-005**: The reconciler unit test suite passes in a headless CI environment
  (no display server required) on all three platforms.
- **SC-006**: Memory usage does not grow across 1000 consecutive full-subtree
  create/destroy cycles in a test harness.

## Assumptions

- The initial platform renderer implementation targets macOS first; Windows and Linux
  backends will be scaffolded but may be partially implemented at v0.1.
- Consumer-facing API uses Rust structs and traits; no macro DSL is required for v1
  (a macro layer may be added later per Principle V / YAGNI).
- Layout is handled by the framework's own layout engine (not delegated to each
  platform's native layout system), to ensure consistent cross-platform behaviour.
- The event loop is driven by each platform's native event loop; `rn` wraps it rather
  than replacing it.
- Accessibility (a11y) exposure through native controls is inherited from the platform
  widgets automatically; no separate a11y API is required for v1.
