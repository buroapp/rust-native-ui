# Tasks: Platform Parity — Windows and Linux Backends

**Input**: Design documents from `/specs/003-platform-parity/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/backend-contract.md

**Tests**: Included only where the specification makes them normative — FR-003 requires the capability declaration to be build-checkable and SC-002 requires 100% coverage enforced by the build. No TDD tasks are generated for control behaviour; that is verified through the running applications per quickstart.md.

**Organization**: Tasks are grouped by user story. US1 and US2 are both P1 but are sequenced, not parallel: Windows completes before Linux, per plan.md "Implementation Sequencing". The reason is that both backends share no code beyond `rn-core`, so running them concurrently doubles the surface being debugged at once without shortening either path.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1–US5, mapping to spec.md
- Exact file paths in every description

## Path Conventions

Cargo workspace at repository root. Backend crates `rn-windows/` and `rn-linux/` already exist as workspace members with feature wiring in `rn/Cargo.toml`; this feature fills them in. Shared crates `rn-core/`, `rn-layout/`, `rn/` are modified only where a task says so.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Dependency and module scaffolding for both backends

- [X] T001 Add Win32 feature set to the `windows` dependency in `/Users/I779886/Developer/rust-native-ui/Cargo.toml` — `Win32_Foundation`, `Win32_UI_Controls`, `Win32_UI_WindowsAndMessaging`, `Win32_Graphics_Gdi`, `Win32_System_LibraryLoader`, `Win32_UI_HiDpi`
- [X] T002 [P] Add `gtk4` (with `v4_8` feature) to workspace dependencies in `/Users/I779886/Developer/rust-native-ui/Cargo.toml` and wire it into `/Users/I779886/Developer/rust-native-ui/rn-linux/Cargo.toml`
- [ ] T003 [P] Create module skeleton in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/` — declare `app`, `renderer`, `capabilities`, `controls` in `lib.rs`, create `controls/mod.rs` and the seven family modules
- [ ] T004 [P] Create module skeleton in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/` — declare `app`, `renderer`, `capabilities`, `widgets` in `lib.rs`, create `widgets/mod.rs` and the seven family modules
- [ ] T005 Verify exactly-one-backend gating compiles: `cargo build -p rn --features windows` and `--features linux` both succeed, and `cargo build -p rn --features windows,macos` fails loudly. Add a compile error if two backend features are enabled, in `/Users/I779886/Developer/rust-native-ui/rn/src/lib.rs` (FR-007)

**Checkpoint**: Both backend crates compile as empty shells

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The completeness mechanism and the render-path storage both backends depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T006 Add `pub const COUNT: usize` to `ComponentKind` and a `pub const ALL: &[ComponentKind]` slice in `/Users/I779886/Developer/rust-native-ui/rn-core/src/lib.rs`, kept in sync by a unit test that asserts `ALL.len() == COUNT` and that every variant appears exactly once
- [X] T007 [P] Add a shared capability-coverage test helper as a macro or function in `/Users/I779886/Developer/rust-native-ui/rn-core/src/lib.rs` (test-only), asserting that a backend's `capabilities()` contains every kind in `ComponentKind::ALL` exactly once
- [X] T008 [P] Define `ControlRecord` and the dense `Vec<Option<ControlRecord>>` store with geometric growth in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/renderer.rs` per research R3 and data-model ControlRecord
- [X] T009 [P] Define the equivalent store in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/renderer.rs`
- [X] T010 Add display detection helper for Linux tests in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/app.rs` — returns whether `WAYLAND_DISPLAY` or `DISPLAY` is set, used to skip windowed tests with a printed reason per research R5

**Checkpoint**: Completeness machinery exists; both stores compile; no controls implemented yet

---

## Phase 3: User Story 1 — Native Windows controls (Priority: P1) 🎯 MVP

**Goal**: An application written against the public API compiles and runs on Windows with every component drawn by a Win32 control.

**Independent Test**: `cargo run -p counter --features rn/windows` and `cargo run -p settings-demo --features rn/windows` both open a non-blank window whose controls respond to input, with zero edits to `examples/`.

### Window, message pump and scaling

- [ ] T011 [US1] Implement window class registration, message pump and `run_event_loop` in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/app.rs` — `RegisterClassW`, `CreateWindowExW`, `GetMessageW`/`DispatchMessageW`, clean exit when the last window closes
- [ ] T012 [US1] Implement per-monitor DPI awareness v2: application manifest entry plus `WM_DPICHANGED` handling that re-applies the last layout at the new scale factor, in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/app.rs` (FR-015, research R6)

### Control families (parallelizable — separate files)

- [ ] T013 [P] [US1] Implement `Label`, `Button`, `TextInput`, `TextArea`, `Checkbox`, `Separator` in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/controls/basic.rs` using `STATIC`, `BUTTON`, `EDIT`
- [ ] T014 [P] [US1] Implement `Slider`, `ProgressBar`, `Stepper`, `ActivityIndicator`, `CapacityIndicator`, `RelevanceIndicator` in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/controls/ranged.rs` using `msctls_trackbar32`, `msctls_progress32`, `msctls_updown32`
- [ ] T015 [P] [US1] Implement `Dropdown`, `PopupButton`, `SegmentedControl`, `RadioGroup`, `Switch` in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/controls/choice.rs` using `COMBOBOX` and `BUTTON` style variants per data-model Windows mapping
- [ ] T016 [P] [US1] Implement leaf controls `Image`, `ColorWell`, `DatePicker`, `TimePicker`, `GraphicalTimePicker`, `RichText`, `Badge`, `Skeleton`, `IconButton`, `BackButton`, `DisclosureButton`, `HelpButton`, `RatingIndicator` in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/controls/leaf.rs` — note this file is added to the skeleton from T003
- [ ] T017 [P] [US1] Implement containers `LayoutContainer`, `Card`, `GroupedSection`, `ContentArea`, `Scaffold`, `SafeAreaView`, `TouchableOpacity`, `SplitView` in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/controls/containers.rs`
- [ ] T018 [P] [US1] Implement `FlatList` with `LVS_OWNERDATA` virtual mode, `ScrollView`, `ListTile`, `Sidebar`, `SidebarItem` in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/controls/lists.rs` (FR-011, SC-006)
- [ ] T019 [P] [US1] Implement chrome `Window`, `TitleBar`, `Toolbar`, `ToolbarIconButton`, `ToolbarSpacer`, `ToolbarDivider`, `ToolbarPopup`, `StatusBar` in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/controls/chrome.rs`
- [ ] T020 [P] [US1] Implement `Modal`, `Popover`, `AlertDialog` via `TaskDialogIndirect`, `Sheet`, `Tooltip` in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/controls/dialogs.rs`

### Wiring, state and events

- [ ] T021 [US1] Implement the exhaustive `ComponentKind` match in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/renderer.rs` dispatch to the family modules, with no `_` catch-all arm (FR-003, contract C4) — depends on T013–T020
- [ ] T022 [US1] Implement the property-update table in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/controls/mod.rs` covering every row of data-model "Cross-platform state coverage" (FR-009)
- [ ] T023 [US1] Implement delete and reorder operations in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/renderer.rs` — destroy handles, clear slots, reposition without recreate (contract C2)
- [ ] T024 [US1] Implement event emission in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/app.rs` — `WM_COMMAND`, `WM_NOTIFY`, `WM_HSCROLL`, `WM_CTLCOLOR*` mapped to the payload shapes in contract C5, including touchable bubbling that walks the shadow parent chain
- [ ] T025 [US1] Implement `capabilities()` in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/capabilities.rs` listing all 57 kinds from data-model Windows mapping, plus the T007 coverage test (SC-002)

**Checkpoint**: Windows backend complete. `counter`, `gallery`, `showcase`, `settings-demo` and `mail-demo` all run non-blank on Windows with no `examples/` changes.

---

## Phase 4: User Story 2 — Native Linux controls (Priority: P1)

**Goal**: The same applications compile and run on Linux under both Wayland and X11 with every component drawn by a GTK4 widget.

**Independent Test**: `cargo run -p settings-demo --features rn/linux` under both `GDK_BACKEND=x11` and `GDK_BACKEND=wayland` opens a non-blank window whose controls respond to input, with zero edits to `examples/`.

**Dependency**: Sequenced after US1 by plan.md. Not strictly blocked — the crates are independent — but debugging two new backends at once is slower than finishing one.

### Application and scaling

- [ ] T026 [US2] Implement `GtkApplication` startup, window listing and `run_event_loop` in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/app.rs` with no assumption about Wayland vs X11
- [ ] T027 [US2] Implement dark/light preference tracking via `gtk4::Settings::gtk-application-prefer-dark-theme` in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/app.rs` so controls follow the desktop theme live (FR-014)

### Widget families (parallelizable — separate files)

- [ ] T028 [P] [US2] Implement `Label`, `Button`, `TextInput`, `TextArea`, `Checkbox`, `Separator` in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/widgets/basic.rs` using `GtkLabel`, `GtkButton`, `GtkEntry`, `GtkTextView`, `GtkCheckButton`, `GtkSeparator`
- [ ] T029 [P] [US2] Implement `Slider` via `GtkScale`, `ProgressBar` via `GtkProgressBar`, `Stepper` via `GtkSpinButton`, `ActivityIndicator` via `GtkSpinner`, `CapacityIndicator` and `RelevanceIndicator` via `GtkLevelBar` in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/widgets/ranged.rs`
- [ ] T030 [P] [US2] Implement `Dropdown` via `GtkDropDown`, `PopupButton` and `HelpButton` via `GtkMenuButton`, `SegmentedControl` via linked `GtkToggleButton` group, `RadioGroup` via `GtkCheckButton` grouping, `Switch` via `GtkSwitch` in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/widgets/choice.rs`
- [ ] T031 [P] [US2] Implement leaf widgets `Image`, `ColorWell` via `GtkColorButton`, `DatePicker` via `GtkMenuButton`+`GtkPopover`+`GtkCalendar`, `TimePicker` and `GraphicalTimePicker` via spin-button composition, `RichText` via `GtkTextTag`, `Badge`, `Skeleton`, `IconButton`, `BackButton`, `DisclosureButton`, `RatingIndicator` in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/widgets/leaf.rs` — added to skeleton from T004
- [ ] T032 [P] [US2] Implement containers `LayoutContainer` via `GtkBox`, `Card`/`GroupedSection` via CSS `card`, `ContentArea`, `Scaffold`, `SafeAreaView`, `TouchableOpacity` via `GtkGestureClick`, `SplitView` via `GtkPaned` in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/widgets/containers.rs`
- [ ] T033 [P] [US2] Implement `FlatList` via `GtkListView` + `GtkSingleSelection`, `ScrollView` via `GtkScrolledWindow`, `ListTile`/`SidebarItem` via `GtkListBoxRow`, `Sidebar` via `GtkListBox` in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/widgets/lists.rs` (FR-011, SC-006)
- [ ] T034 [P] [US2] Implement chrome `Window`, `TitleBar` and `Toolbar` via `GtkHeaderBar`, `ToolbarIconButton`/`ToolbarPopup` packed into the header bar, `ToolbarSpacer` with `hexpand`, `ToolbarDivider` via `GtkSeparator`, `StatusBar` via `GtkActionBar` in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/widgets/chrome.rs`
- [ ] T035 [P] [US2] Implement `Modal` via modal `GtkWindow`, `Popover` via `GtkPopover`, `AlertDialog` via `GtkAlertDialog` with `GtkMessageDialog` fallback for GTK 4.8, `Sheet` via transient window, `Tooltip` via `set_tooltip_text` in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/widgets/dialogs.rs`

### Wiring, state and events

- [ ] T036 [US2] Implement the exhaustive `ComponentKind` match in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/renderer.rs` dispatch to the family modules, with no `_` catch-all arm (FR-003, contract C4) — depends on T028–T035
- [ ] T037 [US2] Implement the property-update table in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/widgets/mod.rs` covering every row of data-model "Cross-platform state coverage" (FR-009)
- [ ] T038 [US2] Implement delete and reorder in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/renderer.rs` — unparent widgets, clear slots, reorder without recreate (contract C2)
- [ ] T039 [US2] Implement event emission in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/widgets/mod.rs` — `connect_clicked`, `connect_changed`, `connect_activate`, `connect_value_changed` mapped to contract C5 payloads, including touchable bubbling
- [ ] T040 [US2] Implement `capabilities()` in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/capabilities.rs` listing all 57 kinds from data-model Linux mapping, plus the T007 coverage test (SC-002)

**Checkpoint**: Linux backend complete. All five applications run non-blank under both display protocols with no `examples/` changes.

---

## Phase 5: User Story 3 — Layout identical across platforms (Priority: P2)

**Goal**: Node geometry agrees across platforms within two logical pixels.

**Independent Test**: Render an identical component tree at an identical window size on all three platforms, diff the computed bounds, confirm no node differs by more than 2 logical pixels.

- [ ] T041 [US3] Add a debug layout-dump mode to the Windows backend in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/renderer.rs` routing through `log::debug!` in the same format the macOS backend already emits
- [ ] T042 [P] [US3] Add the equivalent layout-dump mode in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/renderer.rs`
- [X] T043 [US3] Add a cross-platform geometry assertion test in `/Users/I779886/Developer/rust-native-ui/rn-layout/tests/cross_platform.rs` proving `compute_layout` is platform-independent — run the same spec twice and assert identical results, establishing that any divergence observed in practice comes from the backend, not the engine
- [ ] T044 [US3] Audit both new backends for geometry computation outside the shared layout result — grep for arithmetic on positions and sizes in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/` and `/Users/I779886/Developer/rust-native-ui/rn-linux/src/`, remove any found. Permitted transformation is the Y-axis flip only (contract C3, FR-008)

**Checkpoint**: Layout is provably shared; backends only apply it

---

## Phase 6: User Story 4 — Automated cross-platform verification (Priority: P2)

**Goal**: CI builds and tests on all three platforms and fails when a backend is incomplete.

**Independent Test**: Introduce a throwaway `ComponentKind` variant and confirm both new backends fail to build; introduce a Windows-only regression and confirm only the Windows job fails.

- [X] T045 [US4] Create `/Users/I779886/Developer/rust-native-ui/.github/workflows/ci.yml` with a three-platform matrix (`macos-latest`, `windows-latest`, `ubuntu-latest`), each running `cargo build --workspace --all-targets` and `cargo test --workspace` with its own backend feature
- [X] T046 [US4] Add GTK4 system dependency installation and `xvfb-run` wrapping to the Linux job in `/Users/I779886/Developer/rust-native-ui/.github/workflows/ci.yml` so windowed tests run rather than skip (research R5)
- [X] T047 [US4] Add the completeness guard job to `/Users/I779886/Developer/rust-native-ui/.github/workflows/ci.yml` — a step that appends a throwaway variant to `ComponentKind`, asserts both backends fail to build, then reverts. This is the only way to prove FR-003 stays true as the enum grows
- [X] T048 [P] [US4] Add `cargo clippy --workspace --all-targets -- -D warnings` to all three matrix legs in `/Users/I779886/Developer/rust-native-ui/.github/workflows/ci.yml`
- [X] T049 [P] [US4] Add a Wayland verification job on a nightly schedule to `/Users/I779886/Developer/rust-native-ui/.github/workflows/ci.yml` using a Wayland compositor, since the standard runners provide only X11 via Xvfb (SC-008)

**Checkpoint**: Every PR produces build and test results for all three platforms with no manual dispatch

---

## Phase 7: User Story 5 — Native accessibility (Priority: P3)

**Goal**: Every interactive control exposes a role and name; keyboard traversal reaches all of them.

**Independent Test**: Inspect the accessibility tree of a running application on each platform; confirm every interactive control has a non-empty role and name, and that Tab traversal reaches each control in tree order.

- [ ] T050 [US5] Map component semantics to UI Automation in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/app.rs` — set accessible names from the associated `Label` text via `WM_GETOBJECT`/`IAccessible` handling, and expose control roles (FR-017, FR-018)
- [ ] T051 [P] [US5] Map component semantics to GTK4 accessibility in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/widgets/mod.rs` — `update_property` with `AccessibleProperty::Label` from the associated label text, and ensure custom-composed widgets set `accessible-role` (FR-017, FR-018)
- [ ] T052 [US5] Ensure `WS_TABSTOP` is set correctly on every interactive control in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/controls/mod.rs` so Tab traversal matches tree order (FR-019)
- [ ] T053 [P] [US5] Ensure `set_focusable` and `set_can_focus` are set correctly on every interactive widget, and update the focus chain in tree order, in `/Users/I779886/Developer/rust-native-ui/rn-linux/src/widgets/mod.rs` (FR-019)
- [ ] T054 [US5] Verify accessibility with Inspect.exe on Windows and Accerciser on Linux against `settings-demo`, documenting the observed role and name for every interactive control in `/Users/I779886/Developer/rust-native-ui/specs/003-platform-parity/quickstart.md` under section 7

**Checkpoint**: Accessibility tree inspection and keyboard-only traversal both pass on all three platforms

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Constitution gates and end-to-end validation

- [ ] T055 [P] Measure render budget on Windows: time `apply` plus event dispatch against the constitution's baseline hardware, confirm under 8 ms, record the measurement in `/Users/I779886/Developer/rust-native-ui/specs/003-platform-parity/research.md` (constitution IV)
- [ ] T056 [P] Measure render budget on Linux on Ryzen 5 5600G-class hardware and record alongside T055 in `/Users/I779886/Developer/rust-native-ui/specs/003-platform-parity/research.md` (constitution IV)
- [ ] T057 Audit both new backends for `unwrap`, `expect` and `panic` in non-test code in `/Users/I779886/Developer/rust-native-ui/rn-windows/src/` and `/Users/I779886/Developer/rust-native-ui/rn-linux/src/`, replacing each with a diagnostic `Err` path (FR-021, contract C6)
- [X] T058 [P] Verify no platform type leaks into the public API — confirm `rn/src/lib.rs` exports no `HWND`, `GtkWidget`, or `gtk4::` type in its public surface (FR-006, constitution III)
- [ ] T059 Run the full quickstart.md checklist end to end on Windows and Linux, recording the observed result for each of the twelve boxes in `/Users/I779886/Developer/rust-native-ui/specs/003-platform-parity/quickstart.md`
- [ ] T060 Confirm `git diff --stat examples/` is empty for `/Users/I779886/Developer/rust-native-ui/examples/` on the feature branch, proving FR-005 and SC-005 — no application source required a platform conditional

**Checkpoint**: All eight success criteria from spec.md are demonstrably met

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies
- **Foundational (Phase 2)**: depends on Setup; **blocks US1 and US2**
- **US1 Windows (Phase 3)**: depends on Foundational
- **US2 Linux (Phase 4)**: depends on Foundational. Sequenced after US1 by choice, not by technical constraint — the crates are independent, but the plan resolves to finish one before starting the other
- **US3 Geometry (Phase 5)**: needs at least one new backend to exist to have anything to compare; practically depends on US1
- **US4 CI (Phase 6)**: T045–T046 and T048 can land any time after Setup; T047 (the completeness guard) is only meaningful once at least one backend matches exhaustively, so after US1
- **US5 Accessibility (Phase 7)**: depends on US1 for the Windows half, US2 for the Linux half
- **Polish (Phase 8)**: depends on all stories being complete

### User Story Dependencies

- **US1 (P1)**: independent after Foundational. Delivers a shippable Windows backend on its own
- **US2 (P1)**: independent after Foundational. Delivers a shippable Linux backend on its own
- **US3 (P2)**: consumes US1 output as its comparison reference
- **US4 (P2)**: partly independent; the guard task T047 depends on US1
- **US5 (P3)**: split across US1 and US2 by platform

### Within Each Story

- Family control modules T013–T020 (Windows) and T028–T035 (Linux) are mutually parallel — different files, no overlap
- The exhaustive dispatch (T021, T036) depends on every family module, because a missing family is a missing match arm and will not compile
- State table and event wiring depend on the dispatch existing
- `capabilities()` and its coverage test come last, since they enumerate what actually got implemented

### Parallel Opportunities

- T002–T004 run in parallel after T001
- T008 and T009 (the two control stores) are parallel
- T013–T020: all eight Windows family modules in parallel
- T028–T035: all eight Linux family modules in parallel
- T041 and T042 (layout dump modes) are parallel
- T048 and T049 are parallel to the rest of US4
- T050 and T051, T052 and T053 are parallel pairs
- T055 and T056, T058 are parallel

---

## Parallel Example: User Story 1 control families

```bash
# After T011, T012 and the Phase 2 checkpoint, launch all Windows families together:
Task: "Implement basic.rs controls in rn-windows/src/controls/basic.rs"
Task: "Implement ranged.rs controls in rn-windows/src/controls/ranged.rs"
Task: "Implement choice.rs controls in rn-windows/src/controls/choice.rs"
Task: "Implement leaf.rs controls in rn-windows/src/controls/leaf.rs"
Task: "Implement containers.rs controls in rn-windows/src/controls/containers.rs"
Task: "Implement lists.rs controls in rn-windows/src/controls/lists.rs"
Task: "Implement chrome.rs controls in rn-windows/src/controls/chrome.rs"
Task: "Implement dialogs.rs controls in rn-windows/src/controls/dialogs.rs"
```

```bash
# Similarly for Linux after T026, T027:
Task: "Implement basic.rs widgets in rn-linux/src/widgets/basic.rs"
Task: "Implement ranged.rs widgets in rn-linux/src/widgets/ranged.rs"
Task: "Implement choice.rs widgets in rn-linux/src/widgets/choice.rs"
Task: "Implement leaf.rs widgets in rn-linux/src/widgets/leaf.rs"
Task: "Implement containers.rs widgets in rn-linux/src/widgets/containers.rs"
Task: "Implement lists.rs widgets in rn-linux/src/widgets/lists.rs"
Task: "Implement chrome.rs widgets in rn-linux/src/widgets/chrome.rs"
Task: "Implement dialogs.rs widgets in rn-linux/src/widgets/dialogs.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1 Setup — T001–T005
2. Phase 2 Foundational — T006–T010 (blocks everything)
3. Phase 3 US1 — T011–T025
4. **STOP and VALIDATE**: run all five applications on Windows; confirm non-blank, controls respond, `examples/` unchanged
5. That is a shippable Windows backend

### Incremental Delivery

1. Setup + Foundational → shells compile
2. US1 → **Windows ships**. The framework is genuinely two-platform
3. US2 → **Linux ships**. The framework is three-platform
4. US3 → layout divergence becomes a caught bug instead of a report
5. US4 → parity is enforced going forward rather than maintained by memory
6. US5 → accessible on all three
7. Polish → constitution gates measured, not assumed

### Sequencing note

The plan.md decision stands: finish Windows before starting Linux. The two backends do not share code, so parallelising them does not shorten the critical path — it just interleaves two unfamiliar toolkits in the same working memory. If a second engineer is available, US2 can start after the Phase 2 checkpoint, because the only shared prerequisites are T006, T007 and the store pattern in T009.

---

## Notes

- `[P]` marks different files with no dependency — verify before running two tasks at once
- Adding a file to a module skeleton (T016, T031) also means adding it to the `mod` declaration from T003/T004
- The exhaustive match means a partially finished backend **will not compile**. That is intended. Work on a family module until it compiles, or the tree stays broken
- Commit after each family module lands
- Stop at each checkpoint and validate independently — a checkpoint that was not validated is not a checkpoint
- T060 (`examples/` unchanged) is the cheapest test in the list and catches the most common failure: quietly editing a demo to make a backend work
