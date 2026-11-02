---

description: "Task list template for feature implementation"
---

# Tasks: Core Framework (001-core-framework)

**Input**: Design documents from `specs/001-core-framework/`

**Prerequisites**: plan.md ✅ spec.md ✅ research.md ✅ data-model.md ✅ contracts/ ✅

**Tests**: No TDD requested. Headless reconciler unit tests included in Phase 4
as implementation tasks (they are the verification mechanism for US2, not a
separate TDD phase).

**Organization**: Tasks grouped by user story to enable independent implementation
and testing of each story.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Exact file paths included in every task description

---

## Phase 1: Setup

**Purpose**: Cargo workspace initialization and skeleton crate structure.

- [ ] T001 Create Cargo workspace root `Cargo.toml` with members: `rn`, `rn-layout`, `rn-macos`, `rn-windows`, `rn-linux`, `examples/hello-world`, `examples/counter`
- [ ] T002 Create `rn/Cargo.toml` with deps: `crossbeam-channel`, and `rn-layout` as path dep; add feature flags `macos`/`windows`/`linux` each enabling their respective backend crate
- [ ] T003 [P] Create `rn-layout/Cargo.toml` with dep: `taffy`
- [ ] T004 [P] Create `rn-macos/Cargo.toml` with deps: `objc2`, `objc2-app-kit`, `rn` as path dep; `[target.'cfg(target_os="macos")']`
- [ ] T005 [P] Create `rn-windows/Cargo.toml` with dep: `windows` crate; `[target.'cfg(target_os="windows")']`
- [ ] T006 [P] Create `rn-linux/Cargo.toml` with dep: `gtk4`; `[target.'cfg(target_os="linux")']`
- [ ] T007 [P] Create `examples/hello-world/Cargo.toml` with dep: `rn` as path dep
- [ ] T008 [P] Create `examples/counter/Cargo.toml` with dep: `rn` as path dep
- [ ] T009 [P] Create empty `src/lib.rs` (or `src/main.rs`) stub files for all crates so workspace builds clean

---

## Phase 2: Foundational

**Purpose**: Core types and reconciler — shared infrastructure that MUST be complete
before any user story can be implemented.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [ ] T010 Define `NodeId`, `ComponentKind`, `Props` (`PropKey`, `PropValue`), `Component`, `ComponentTree` in `rn/src/component.rs` per data-model.md
- [ ] T011 [P] Define `Event`, `EventKind`, `EventPayload`, `HandlerId` in `rn/src/event.rs` per data-model.md
- [ ] T012 Define `Operation` enum (Create/Update/Delete/Reorder) and `Changeset` struct in `rn/src/reconciler.rs` per data-model.md
- [ ] T013 Define `PlatformRenderer` trait (`apply`, `run_event_loop`) in `rn/src/renderer.rs` per contracts/public-api.md
- [ ] T014 Define `RnError`, `RenderError`, `ReconcileError` error types in `rn/src/error.rs` per contracts/public-api.md
- [ ] T015 Implement `Reconciler` struct with `previous: Option<ComponentTree>` and `node_map` in `rn/src/reconciler.rs`; implement `reconcile(&mut self, next: ComponentTree) -> Result<Changeset, ReconcileError>` using position-based identity heuristic (research.md: Reconciler Algorithm)
- [ ] T016 Implement explicit key support in `Reconciler::reconcile` — detect keyed nodes, emit `Reorder` instead of Delete+Create when key matches a sibling in `rn/src/reconciler.rs`
- [ ] T017 [P] Implement `compute_layout(tree: &ComponentTree) -> LayoutResult` using `taffy` in `rn-layout/src/lib.rs`; `LayoutResult` maps `NodeId` to absolute pixel rect
- [ ] T018 Implement builder API (`rn::window()`, `rn::label()`, `rn::button()`, `rn::text_input()`, `rn::vstack()`, `rn::hstack()`, `rn::keyed()`) in `rn/src/lib.rs` per contracts/public-api.md
- [ ] T019 Implement `rn::on(id: HandlerId, handler: impl Fn(Event) + Send + 'static)` global handler registry in `rn/src/lib.rs`
- [ ] T020 Implement `rn::render<R: PlatformRenderer>(renderer: &mut R, tree: ComponentTree) -> Result<(), RnError>` in `rn/src/lib.rs` — calls reconciler then `renderer.apply(changeset)`
- [ ] T021 Implement `rn::run<R: PlatformRenderer>(renderer: &mut R) -> Result<(), RnError>` in `rn/src/lib.rs` — delegates to `renderer.run_event_loop()`

**Checkpoint**: `cargo check -p rn` and `cargo check -p rn-layout` pass clean.
Foundation ready — user story implementation can begin.

---

## Phase 3: User Story 1 — Write Once, Run Native Everywhere (Priority: P1) 🎯 MVP

**Goal**: A window with a label and a button compiles and renders using native OS
controls on macOS. Windows and Linux backends are scaffolded (compile, no-op apply).

**Independent Test**: Run `cargo run --example hello-world --features rn/macos`.
A native macOS window opens with AppKit controls. Verify via Accessibility Inspector
(roles: AXWindow, AXStaticText, AXButton). Windows and Linux scaffold: `cargo check
--target x86_64-pc-windows-msvc` and `cargo check --target x86_64-unknown-linux-gnu`
pass.

### Implementation for User Story 1

- [ ] T022 [US1] Implement `MacosRenderer::new() -> Result<Self, RenderError>` — initialise NSApplication in `rn-macos/src/renderer.rs`
- [ ] T023 [US1] Implement `MacosRenderer::apply_create()` for `ComponentKind::Window` — create `NSWindow` via `objc2-app-kit` in `rn-macos/src/renderer.rs`; store handle in internal `node_map: HashMap<NodeId, NativeHandle>`
- [ ] T024 [P] [US1] Implement `MacosRenderer::apply_create()` for `ComponentKind::Label` — create `NSTextField` (non-editable) in `rn-macos/src/renderer.rs`
- [ ] T025 [P] [US1] Implement `MacosRenderer::apply_create()` for `ComponentKind::Button` — create `NSButton` in `rn-macos/src/renderer.rs`
- [ ] T026 [P] [US1] Implement `MacosRenderer::apply_create()` for `ComponentKind::TextInput` — create `NSTextField` (editable) in `rn-macos/src/renderer.rs`
- [ ] T027 [P] [US1] Implement `MacosRenderer::apply_create()` for `ComponentKind::LayoutContainer` — create `NSView` container, invoke `rn-layout` to get rects, set subview frames in `rn-macos/src/renderer.rs`
- [ ] T028 [US1] Implement `MacosRenderer::run_event_loop()` — start `NSApplication::run()` main loop, drain `crossbeam-channel` receiver on each tick, dispatch events to `rn::on` handlers in `rn-macos/src/renderer.rs`
- [ ] T029 [P] [US1] Stub `WindowsRenderer` — implement `PlatformRenderer` trait with no-op `apply()` and `run_event_loop()` returning `Ok(())` in `rn-windows/src/renderer.rs`
- [ ] T030 [P] [US1] Stub `LinuxRenderer` — implement `PlatformRenderer` trait with no-op `apply()` and `run_event_loop()` returning `Ok(())` in `rn-linux/src/renderer.rs`
- [ ] T031 [US1] Implement `examples/hello-world/src/main.rs` — construct `ComponentTree` with Window > VStack > [Label("Hello, World!"), Button("Click me")]; call `rn::render` then `rn::run`
- [ ] T032 [US1] Verify `cargo run --example hello-world --features rn/macos` produces a visible native macOS window; inspect with Accessibility Inspector; confirm AXWindow + AXStaticText + AXButton

**Checkpoint**: US1 fully functional and independently testable on macOS.
Windows and Linux scaffold compiles.

---

## Phase 4: User Story 2 — State Change Triggers Minimal Native Update (Priority: P2)

**Goal**: Incrementing a counter updates only the label's NSTextField text prop.
No control is recreated. Reconciler verified headlessly.

**Independent Test**: Run `cargo run --example counter --features rn/macos`. Click
Increment. Label updates without flicker. Run `cargo test -p rn --test reconciler`
headlessly (no display server) — all tests pass.

### Implementation for User Story 2

- [ ] T033 [US2] Implement prop-level diffing in `Reconciler::reconcile` — for nodes that exist in both old and new tree at same position with same kind, emit `Operation::Update` containing only props whose values differ in `rn/src/reconciler.rs`; emit nothing if all props identical
- [ ] T034 [US2] Implement `MacosRenderer::apply_update()` — for each `Operation::Update`, look up native handle by `NodeId`, apply only changed props (e.g. set `NSTextField.stringValue` for `PropKey::Text`) without destroying/recreating the control in `rn-macos/src/renderer.rs`
- [ ] T035 [P] [US2] Implement `MacosRenderer::apply_delete()` — remove native control from parent view and release handle from `node_map` in `rn-macos/src/renderer.rs`
- [ ] T036 [P] [US2] Implement `MacosRenderer::apply_reorder()` — reorder subviews in parent `NSView` per `Operation::Reorder` child_order in `rn-macos/src/renderer.rs`
- [ ] T037 [US2] Implement `examples/counter/src/main.rs` — state struct with `count: u32`; Increment button handler increments count, calls `rn::render` with new tree; label shows "Count: {n}"
- [ ] T038 [P] [US2] Implement headless reconciler tests in `tests/reconciler/tests.rs`:
  - `test_no_change_produces_empty_changeset`
  - `test_prop_change_produces_update_only`
  - `test_node_removal_produces_delete`
  - `test_subtree_removal_no_memory_leak` (1000 cycles)
  - `test_keyed_reorder_produces_reorder_not_delete_create`
- [ ] T039 [US2] Run `cargo test -p rn --test reconciler` and confirm all 5 tests pass headlessly

**Checkpoint**: US1 + US2 both independently functional. Counter updates without
flicker. Reconciler test suite passes without display server.

---

## Phase 5: User Story 3 — Platform Types Never Leak to Consumer (Priority: P3)

**Goal**: A crate depending only on `rn` has zero platform-backend symbols in its
namespace. Events arrive as `rn::event::Event`, never as platform types.

**Independent Test**: `cargo tree -p hello-world --edges normal` shows no `objc2`,
`windows`, or `gtk4` entries directly under `hello-world`.

### Implementation for User Story 3

- [ ] T040 [US3] Audit `rn/src/lib.rs` public exports — confirm no `pub use` of any type from `objc2`, `windows`, or `gtk4` crates; fix any leaks found
- [ ] T041 [P] [US3] Add `#![deny(unreachable_pub)]` and review all `pub` items in `rn/src/` to ensure only intended API surface is exported
- [ ] T042 [P] [US3] Add `deny(unused_extern_crates)` to `examples/hello-world/src/main.rs` and `examples/counter/src/main.rs`; confirm both compile without warnings
- [ ] T043 [US3] Run `cargo tree -p hello-world --edges normal` and `cargo tree -p counter --edges normal`; confirm `objc2`, `windows`, `gtk4` do NOT appear in consumer direct deps; document result in `specs/001-core-framework/quickstart.md` Validation 3 section
- [ ] T044 [P] [US3] Verify event type in counter example handler is `rn::event::Event` (not a platform type) — add a compile-time type assertion in `examples/counter/src/main.rs` using `let _: rn::event::Event = e;`

**Checkpoint**: All three user stories independently functional and verified.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, metadata, and final validation across all stories.

- [ ] T045 [P] Write `README.md` at repo root — project name (Rust Native / `rn`), elevator pitch, quickstart snippet (hello-world), platform support table, link to `specs/001-core-framework/quickstart.md`
- [ ] T046 [P] Add crate metadata to `rn/Cargo.toml`: `description`, `license = "MIT OR Apache-2.0"`, `categories = ["gui", "rendering"]`, `keywords = ["native", "gui", "cross-platform"]`
- [ ] T047 Run memory stability validation: `cargo test -p rn --test reconciler -- memory_stability --nocapture`; confirm "Heap delta: 0 bytes" output
- [ ] T048 [P] Run full quickstart.md validation checklist — execute Validations 1–4 from `specs/001-core-framework/quickstart.md` and tick off each passing item
- [ ] T049 [P] Add `.gitignore` (target/, *.DS_Store) and `rust-toolchain.toml` pinning stable channel

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Phase 2 — macOS renderer + hello-world
- **US2 (Phase 4)**: Depends on Phase 3 — reconciler diffing + counter
- **US3 (Phase 5)**: Depends on Phase 3 (can overlap Phase 4) — API audit
- **Polish (Phase 6)**: Depends on all user story phases complete

### User Story Dependencies

- **US1 (P1)**: Starts after Foundational. No dependency on US2/US3.
- **US2 (P2)**: Starts after US1 (needs MacosRenderer to exist for update path).
- **US3 (P3)**: Starts after US1 (needs hello-world example to audit). Independent of US2.

### Within Each User Story

- Renderer `Create` operations (T022–T027) before `run_event_loop` (T028)
- Example app (T031, T037) after renderer operations complete
- Verification task (T032, T039, T043) always last in its phase

### Parallel Opportunities

- All Phase 1 Cargo.toml tasks (T003–T009) run in parallel
- Within Phase 2: T011, T017 run in parallel with T010, T012–T016
- Within Phase 3: T024–T027 (per-widget Create impls) run in parallel
- T029/T030 (Win/Linux stubs) run in parallel with macOS renderer work
- US3 (T041–T044) runs in parallel with US2 (T033–T039)
- All Polish tasks except T047/T048 run in parallel

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks everything)
3. Complete Phase 3: US1 → macOS hello-world with native controls
4. **STOP AND VALIDATE**: Run hello-world, inspect with Accessibility Inspector
5. Demo/commit if ready

### Incremental Delivery

1. Setup + Foundational → workspace compiles clean
2. US1 → native hello-world on macOS → demo
3. US2 → reconciler diffing + counter → demo
4. US3 → API audit → confirmed zero platform leakage
5. Polish → README + metadata → crate publishable

---

## Notes

- `[P]` = different files, no in-flight dependency — safe to parallelise
- `[USn]` maps task to user story for traceability
- Windows/Linux stubs (T029, T030) exist so CI compiles on all platforms from day 1
- Headless reconciler tests (T038) require no display server — run in any CI environment
- Verification tasks (T032, T039, T043, T047) are human-run validations, not automated test commands
