---

description: "Task list template for feature implementation"
---

# Tasks: Core Framework (001-core-framework)

**Input**: Design documents from `specs/001-core-framework/`

**Prerequisites**: plan.md ✅ spec.md ✅ research.md ✅ data-model.md ✅ contracts/ ✅

**Tests**: No TDD requested. Headless reconciler unit tests included in Phase 4 as
implementation tasks (they are the verification mechanism for US2, not a separate TDD
phase). Benchmark harness added in Polish (validates Principle IV).

**Organization**: Tasks grouped by user story to enable independent implementation and
testing of each story.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Exact file paths included in every task description

---

## Phase 1: Setup

**Purpose**: Cargo workspace initialization, lint config, CI matrix.

- [X] T001 Create Cargo workspace root `Cargo.toml` with members: `rn`, `rn-layout`, `rn-macos`, `rn-windows`, `rn-linux`, `examples/hello-world`, `examples/counter`
- [X] T002 Create `rn/Cargo.toml` with deps: `crossbeam-channel`, `log`, and `rn-layout` as path dep; feature flags `macos`/`windows`/`linux` each enabling the respective backend crate
- [X] T003 [P] Create `rn-layout/Cargo.toml` with dep: `taffy`
- [X] T004 [P] Create `rn-macos/Cargo.toml` with deps: `objc2`, `objc2-app-kit`, `log`, `rn` as path dep; `[target.'cfg(target_os="macos")']`
- [X] T005 [P] Create `rn-windows/Cargo.toml` with deps: `windows` crate, `log`; `[target.'cfg(target_os="windows")']`
- [X] T006 [P] Create `rn-linux/Cargo.toml` with deps: `gtk4`, `log`; `[target.'cfg(target_os="linux")']`
- [X] T007 [P] Create `examples/hello-world/Cargo.toml` with deps: `rn` as path dep, `env_logger`
- [X] T008 [P] Create `examples/counter/Cargo.toml` with deps: `rn` as path dep, `env_logger`
- [X] T009 [P] Create empty `src/lib.rs` (or `src/main.rs`) stub files for all crates so workspace builds clean
- [X] T010 [P] Create `clippy.toml` at repo root and add crate-level `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` to `rn/src/lib.rs`, `rn-layout/src/lib.rs`, `rn-macos/src/lib.rs`, `rn-windows/src/lib.rs`, `rn-linux/src/lib.rs` (FR-008 enforcement, analyze U1)
- [X] T011 [P] Create `rust-toolchain.toml` at repo root pinning `channel = "stable"`
- [X] T012 [P] Create `.gitignore` at repo root (target/, .DS_Store, *.swp)
- [X] T013 [P] Create `.github/workflows/ci.yml` with matrix build+test on `macos-latest`, `windows-latest`, `ubuntu-latest` running `cargo check --features rn/<platform>`, `cargo test -p rn --test reconciler`, `cargo clippy -- -D warnings` (constitution dev-workflow requirement, analyze C2)

---

## Phase 2: Foundational

**Purpose**: Core types, reconciler, threading guard, panic-safe dispatch — shared
infrastructure that MUST be complete before any user story work.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T014 Define `NodeId`, `ComponentKind`, `Props` (`PropKey`, `PropValue`), `Component`, `ComponentTree` in `rn/src/component.rs` per data-model.md
- [X] T015 [P] Define `Event`, `EventKind`, `EventPayload`, `HandlerId` in `rn/src/event.rs` per data-model.md
- [X] T016 Define `Operation` enum (Create/Update/Delete/Reorder) and `Changeset` struct in `rn/src/reconciler.rs` per data-model.md
- [X] T017 Define `PlatformRenderer` trait (`apply`, `run_event_loop`) in `rn/src/renderer.rs` per contracts/public-api.md
- [X] T018 Define `RnError` (with `WrongThread` variant), `RenderError` (with `WrongThread` variant), `ReconcileError` in `rn/src/error.rs` per contracts/public-api.md (FR-012)
- [X] T019 Implement `Reconciler` struct with `previous: Option<ComponentTree>` and `node_map` in `rn/src/reconciler.rs`; implement `reconcile(&mut self, next: ComponentTree) -> Result<Changeset, ReconcileError>` using position-based identity heuristic (research.md: Reconciler Algorithm)
- [X] T020 Implement explicit key support in `Reconciler::reconcile` — detect keyed nodes, emit `Reorder` instead of Delete+Create when key matches a sibling in `rn/src/reconciler.rs`
- [X] T021 [P] Implement `compute_layout(tree: &ComponentTree) -> LayoutResult` using `taffy` in `rn-layout/src/lib.rs`; `LayoutResult` maps `NodeId` to absolute pixel rect
- [X] T022 Implement `MainThreadGuard` in `rn/src/thread.rs` — captures main-thread id on first init via `std::thread::current().id()`, exposes `check() -> Result<(), RnError>` returning `WrongThread` from any other thread (FR-011, FR-012)
- [X] T023 Implement global handler registry in `rn/src/lib.rs` using `OnceLock<Mutex<HashMap<HandlerId, Box<dyn Fn(Event) + Send + 'static>>>>`; expose `rn::on(id, handler)` — thread-safe registration (FR-013 contract)
- [X] T024 Implement panic-safe dispatch helper `dispatch_handler(id, event)` in `rn/src/lib.rs` — wraps call in `std::panic::catch_unwind(AssertUnwindSafe(…))`; on panic, logs via `log::error!("handler {id:?} panicked: {payload}")` and continues (FR-013)
- [X] T025 Implement builder API (`rn::window()`, `rn::label()`, `rn::button()`, `rn::text_input()`, `rn::vstack()`, `rn::hstack()`, `rn::keyed()`) in `rn/src/lib.rs` per contracts/public-api.md
- [X] T026 Implement `rn::update_sender<M>() -> crossbeam_channel::Sender<M>` and matching main-loop drain in `rn/src/lib.rs` (FR-011)
- [X] T027 Implement `rn::render<R: PlatformRenderer>(renderer, tree) -> Result<(), RnError>` in `rn/src/lib.rs` — first calls `MainThreadGuard::check()?`, then reconciler, then `renderer.apply`; emits `log::trace!` for timing and `log::debug!` for changeset op counts (FR-011, FR-012, FR-014)
- [X] T028 Implement `rn::run<R: PlatformRenderer>(renderer) -> Result<(), RnError>` in `rn/src/lib.rs` — guards main-thread, drains `update_sender` receiver each tick, delegates to `renderer.run_event_loop()`

**Checkpoint**: `cargo check -p rn` and `cargo check -p rn-layout` pass clean. Threading
guard rejects off-main calls. Handler panics are caught and logged. Foundation ready.

---

## Phase 3: User Story 1 — Write Once, Run Native Everywhere (Priority: P1) 🎯 MVP

**Goal**: A window with a label and a button compiles and renders using native OS
controls on macOS. Windows and Linux backends scaffolded (compile, no-op apply).

**Independent Test**: `cargo run --example hello-world --features rn/macos` opens a
native macOS window with AppKit controls. Verify via Accessibility Inspector (roles:
AXWindow, AXStaticText, AXButton). Scaffold check: `cargo check --target
x86_64-pc-windows-msvc` and `cargo check --target x86_64-unknown-linux-gnu` pass.

### Implementation for User Story 1

- [X] T029 [US1] Implement `MacosRenderer::new() -> Result<Self, RenderError>` in `rn-macos/src/renderer.rs` — initialise `NSApplication`, store main-thread marker, log init via `log::debug!`
- [X] T030 [US1] Implement `MacosRenderer::apply_create()` for `ComponentKind::Window` — create `NSWindow` via `objc2-app-kit` in `rn-macos/src/renderer.rs`; store handle in internal `node_map: HashMap<NodeId, NativeHandle>`
- [X] T031 [P] [US1] Implement `MacosRenderer::apply_create()` for `ComponentKind::Label` — create `NSTextField` (non-editable) in `rn-macos/src/renderer.rs`
- [X] T032 [P] [US1] Implement `MacosRenderer::apply_create()` for `ComponentKind::Button` — create `NSButton` with click action bridged into event channel in `rn-macos/src/renderer.rs`
- [X] T033 [P] [US1] Implement `MacosRenderer::apply_create()` for `ComponentKind::TextInput` — create `NSTextField` (editable) in `rn-macos/src/renderer.rs`
- [X] T034 [P] [US1] Implement `MacosRenderer::apply_create()` for `ComponentKind::LayoutContainer` — create `NSView` container, invoke `rn-layout` to get rects, set subview frames in `rn-macos/src/renderer.rs`
- [X] T035 [US1] Implement `MacosRenderer::run_event_loop()` — start `NSApplication::run()` main loop, drain `crossbeam-channel` receiver each tick, dispatch events via `rn::dispatch_handler` (panic-safe) in `rn-macos/src/renderer.rs`
- [X] T036 [P] [US1] Stub `WindowsRenderer` — implement `PlatformRenderer` trait with no-op `apply()` returning `Ok(())` and `run_event_loop()` returning `Err(RenderError::OsError("windows backend not implemented in v0.1"))` in `rn-windows/src/renderer.rs`
- [X] T037 [P] [US1] Stub `LinuxRenderer` — implement `PlatformRenderer` trait with no-op `apply()` returning `Ok(())` and `run_event_loop()` returning `Err(RenderError::OsError("linux backend not implemented in v0.1"))` in `rn-linux/src/renderer.rs`
- [X] T038 [US1] Implement `examples/hello-world/src/main.rs` — `env_logger::init()`, construct `ComponentTree` with Window > VStack > [Label("Hello, World!"), Button("Click me")]; call `rn::render` then `rn::run`
- [X] T039 [US1] Verify `cargo run --example hello-world --features rn/macos` produces a visible native macOS window; inspect with Accessibility Inspector; confirm AXWindow + AXStaticText + AXButton roles

**Checkpoint**: US1 fully functional and independently testable on macOS. Windows and
Linux scaffolds compile.

---

## Phase 4: User Story 2 — State Change Triggers Minimal Native Update (Priority: P2)

**Goal**: Incrementing a counter updates only the label's text prop. No control is
recreated. Reconciler verified headlessly.

**Independent Test**: `cargo run --example counter --features rn/macos` — click
Increment, label updates without flicker. `cargo test -p rn --test reconciler` passes
headlessly (no display server).

### Implementation for User Story 2

- [X] T040 [US2] Implement prop-level diffing in `Reconciler::reconcile` — for nodes present in both old and new tree at same position with same kind, emit `Operation::Update` containing only props whose values differ in `rn/src/reconciler.rs`; emit nothing if all props identical
- [X] T041 [US2] Implement render coalescing in `rn::run` main loop — if multiple `update_sender` messages arrive in the same event-loop tick, drain all and apply only the final tree to the reconciler in `rn/src/lib.rs` (spec Edge Cases: "state changes faster than event loop", analyze G1)
- [X] T042 [US2] Implement `MacosRenderer::apply_update()` — for each `Operation::Update`, look up native handle by `NodeId`, apply only changed props (e.g. set `NSTextField.stringValue` for `PropKey::Text`) without destroying/recreating the control in `rn-macos/src/renderer.rs`
- [X] T043 [P] [US2] Implement `MacosRenderer::apply_delete()` — remove native control from parent view and release handle from `node_map` in `rn-macos/src/renderer.rs`
- [X] T044 [P] [US2] Implement `MacosRenderer::apply_reorder()` — reorder subviews in parent `NSView` per `Operation::Reorder` child_order in `rn-macos/src/renderer.rs`
- [X] T045 [US2] Implement `examples/counter/src/main.rs` — state struct with `count: u32`; Increment button handler sends an `Increment` message via `rn::update_sender`; main loop drains, increments state, calls `rn::render` with new tree; label shows "Count: {n}"
- [X] T046 [P] [US2] Implement headless reconciler tests in `tests/reconciler/tests.rs`:
  - `test_no_change_produces_empty_changeset`
  - `test_prop_change_produces_update_only`
  - `test_node_removal_produces_delete`
  - `memory_stability` (1000 subtree-removal cycles — asserts heap delta == 0; name matches T058 filter, analyze I1)
  - `test_keyed_reorder_produces_reorder_not_delete_create`
  - `test_render_off_main_thread_returns_wrong_thread` (FR-012)
  - `test_handler_panic_is_caught_and_logged` (FR-013; uses `log` test adapter)
- [X] T047 [US2] Run `cargo test -p rn --test reconciler` and confirm all 7 tests pass headlessly

**Checkpoint**: US1 + US2 both independently functional. Counter updates without
flicker. Reconciler test suite passes without display server, including thread and
panic policy tests.

---

## Phase 5: User Story 3 — Platform Types Never Leak to Consumer (Priority: P3)

**Goal**: A crate depending only on `rn` has zero platform-backend symbols in its
namespace. Events arrive as `rn::event::Event`, never as platform types.

**Independent Test**: `cargo tree -p hello-world --edges normal` shows no `objc2`,
`windows`, or `gtk4` entries directly under `hello-world`.

### Implementation for User Story 3

- [X] T048 [US3] Audit `rn/src/lib.rs` public exports — confirm no `pub use` of any type from `objc2`, `windows`, or `gtk4` crates; fix any leaks found
- [X] T049 [P] [US3] Add `#![deny(unreachable_pub)]` to `rn/src/lib.rs` and review all `pub` items to ensure only intended API surface is exported
- [X] T050 [P] [US3] Add `#![deny(unused_extern_crates)]` to `examples/hello-world/src/main.rs` and `examples/counter/src/main.rs`; confirm both compile without warnings
- [X] T051 [US3] Run `cargo tree -p hello-world --edges normal` and `cargo tree -p counter --edges normal`; confirm `objc2`, `windows`, `gtk4` do NOT appear in consumer direct deps; document result in `specs/001-core-framework/quickstart.md` Validation 3 section
- [X] T052 [P] [US3] Verify event type in counter handler is `rn::event::Event` — add compile-time type assertion `let _: rn::event::Event = e;` in `examples/counter/src/main.rs`

**Checkpoint**: All three user stories independently functional and verified.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Benchmarks, docs, metadata, final validation.

- [X] T053 Add `rn/benches/reconciler.rs` using `criterion` dev-dep — benchmark `Reconciler::reconcile` on a 500-node tree; assert p95 < 8ms on local hardware (constitution Principle IV, analyze C1)
- [X] T054 Run `cargo bench -p rn` and record result in `docs/benchmarks.md`; fail if p95 ≥ 8ms on reference hardware
- [X] T055 [P] Create `docs/backend-status.md` — table of macOS/Windows/Linux widget coverage per widget type with target milestone for Windows/Linux parity (constitution dev-workflow requirement, analyze U2)
- [X] T056 [P] Write `README.md` at repo root — product name (Rust Native / `rn`), elevator pitch (declarative cross-platform native UI, React Native model for Rust), quickstart snippet from hello-world, platform support table, link to `specs/001-core-framework/quickstart.md`; include glossary line: "**Rust Native** is the product; **`rn`** is the crate." (analyze D1)
- [X] T057 [P] Add crate metadata to `rn/Cargo.toml`: `description`, `license = "MIT OR Apache-2.0"`, `categories = ["gui", "rendering"]`, `keywords = ["native", "gui", "cross-platform", "react-native"]`, `repository = "https://github.com/buroapp/rust-native-ui"`
- [X] T058 Run memory stability validation: `cargo test -p rn --test reconciler memory_stability -- --nocapture`; confirm "Heap delta: 0 bytes" output (analyze I1 — test name now matches)
- [X] T059 [P] Run full quickstart.md validation checklist — execute Validations 1–4 from `specs/001-core-framework/quickstart.md` and tick off each passing item

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Phase 2 — macOS renderer + hello-world
- **US2 (Phase 4)**: Depends on Phase 3 (needs MacosRenderer to extend)
- **US3 (Phase 5)**: Depends on Phase 3; can overlap Phase 4
- **Polish (Phase 6)**: Depends on all user story phases complete

### User Story Dependencies

- **US1 (P1)**: Starts after Foundational. No dependency on US2/US3.
- **US2 (P2)**: Starts after US1 (needs MacosRenderer scaffolding).
- **US3 (P3)**: Starts after US1 (needs hello-world example to audit). Independent of US2.

### Within Each User Story

- Renderer `Create` ops (T030–T034) before `run_event_loop` (T035)
- Reconciler diffing (T040) before per-op apply methods (T042–T044)
- Example apps (T038, T045) after their renderer ops land
- Verification (T039, T047, T051) always last in its phase

### Parallel Opportunities

- All Phase 1 Cargo.toml tasks (T003–T013) run in parallel
- Within Phase 2: T015, T021 run in parallel with T014, T016–T020
- Within Phase 3: T031–T034 (per-widget Create impls) run in parallel
- T036/T037 (Win/Linux stubs) run in parallel with macOS renderer work
- Within Phase 4: T043, T044, T046 run in parallel with T042/T045
- US3 (T049–T052) runs in parallel with US2 (T040–T047)
- Phase 6 docs (T055, T056, T057, T059) run in parallel; T053/T054/T058 serial on bench results

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (CI matrix compiles clean on all 3 platforms from day 1)
2. Complete Phase 2: Foundational (CRITICAL — blocks everything)
3. Complete Phase 3: US1 → macOS hello-world with native controls
4. **STOP AND VALIDATE**: Run hello-world, inspect with Accessibility Inspector
5. Demo/commit if ready

### Incremental Delivery

1. Setup + Foundational → workspace compiles clean on 3 platforms (CI green)
2. US1 → native hello-world on macOS → demo
3. US2 → reconciler diffing + counter + headless tests → demo
4. US3 → API audit → confirmed zero platform leakage
5. Polish → benchmarks + README + metadata → crate publishable

---

## Notes

- `[P]` = different files, no in-flight dependency — safe to parallelise
- `[USn]` maps task to user story for traceability
- Windows/Linux stubs (T036, T037) exist so CI compiles on all platforms from day 1
- Headless reconciler tests (T046) require no display server — run in any CI job
- Verification tasks (T039, T047, T051, T054, T058, T059) are measured validations,
  not plain `cargo test` passes
- FR-011..FR-014 (threading, panic policy, observability) covered by T022–T024,
  T026–T028, T046 (thread + panic tests)
- Analyze findings: C1→T053/T054, C2→T013, I1→T046/T058, U1→T010, U2→T055, G1→T041, D1→T056
