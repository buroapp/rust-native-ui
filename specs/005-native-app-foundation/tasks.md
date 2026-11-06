# Tasks: Native App Foundation — Phase A

**Feature**: `005-native-app-foundation`
**Input**: Design documents from `specs/005-native-app-foundation/`
**Scope**: Phase A only — US1 (commands), US2 (table), US3 (tree). Phases B and C follow in a separate tasks file after Phase A ships.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- File paths are relative to the workspace root

---

## Phase 1: Setup

**Purpose**: Create the new source files and workspace member so the workspace compiles throughout. All setup tasks can run before any models are written.

- [X] T001 Create `rn-core/src/data/` module — add `mod.rs`, `command.rs`, `table.rs`, `tree.rs` as empty files and declare `pub mod data;` in `rn-core/src/lib.rs`
- [X] T002 Add `ComponentKind::Table` and `ComponentKind::Tree` to the `ComponentKind` enum in `rn-core/src/lib.rs` (no arms yet; they will fail the completeness guard until T014)
- [X] T003 [P] Create `rn-macos/src/commands.rs` as an empty module and declare `mod commands;` in `rn-macos/src/lib.rs`
- [X] T004 [P] Create `rn-macos/src/table.rs` as an empty module and declare `mod table;` in `rn-macos/src/lib.rs`
- [X] T005 [P] Create `rn-macos/src/tree.rs` as an empty module and declare `mod tree;` in `rn-macos/src/lib.rs`
- [X] T006 Create `examples/table-demo/` — `Cargo.toml` (package `table-demo`, deps `rn`, features `macos`/`windows`/`linux`) and a stub `src/main.rs` that prints "table-demo not implemented yet"; add `examples/table-demo` to workspace members in `Cargo.toml`

**Checkpoint**: `cargo build --workspace` exits 0. The new module files exist and the new kinds compile (they will not yet be handled in renderer or stubs).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Data models in `rn-core`, new prop keys/values, reconciler reset arms, completeness obligations. Everything user-story work depends on. No story label.

**⚠️ CRITICAL**: Nothing in US1, US2 or US3 can begin until this phase is complete.

- [X] T007 Implement identity newtypes in `rn-core/src/data/mod.rs`: `CommandId(u64)`, `ColumnId(u64)`, `RowId(u64)`, `FieldId(u64)`, `FlagId(u64)` — all derive `Debug, Clone, Copy, PartialEq, Eq, Hash`; add `pub use data::{CommandId, ColumnId, RowId, FieldId, FlagId};` to `rn-core/src/lib.rs`
- [X] T008 [P] Implement command data types in `rn-core/src/data/command.rs`: `Modifiers` (bitset u8 with associated consts Command/Shift/Option/Control), `KeyBinding { key: String, modifiers: Modifiers }`, `AvailabilityRule` (enum: `Always`, `WhenFlag(FlagId)`, `Never`), `CommandPlacement { menu: String, order: u32 }`, `Command { id: CommandId, title: String, shortcut: Option<KeyBinding>, available: AvailabilityRule, handler: HandlerId, placement: CommandPlacement }`, `CommandSet(Vec<Command>)`; re-export from `rn-core/src/lib.rs`
- [X] T009 [P] Implement table data types in `rn-core/src/data/table.rs`: `TextAlign` (Left/Center/Right), `TableColumn { id: ColumnId, title: String, width: f32, min_width: f32, alignment: TextAlign, sortable: bool }`, `TableCell { text: String, icon: Option<String> }`, `TableRow { id: RowId, cells: Vec<TableCell>, selectable: bool }`, `SortDirection` (Ascending/Descending), `SortDescriptor { column: ColumnId, direction: SortDirection }`, `Selection { selected: Vec<RowId>, anchor: Option<RowId> }`, `TableModel { columns: Vec<TableColumn>, rows: Vec<TableRow>, sort: Option<SortDescriptor>, selection: Selection }`; add builder/constructor `TableModel::new(columns, rows)` that validates cell count; re-export from `rn-core/src/lib.rs`
- [X] T010 [P] Implement tree data types in `rn-core/src/data/tree.rs`: `TreeNode { id: RowId, label: String, icon: Option<String>, children: Vec<TreeNode>, expanded: bool }`, `TreeModel { roots: Vec<TreeNode>, selection: Option<RowId> }`; add `TreeModel::find_expanded() -> Vec<RowId>` helper for restoration; re-export from `rn-core/src/lib.rs`
- [X] T011 Add new `PropKey` variants to `rn-core/src/lib.rs`: `Commands`, `TableModel`, `TreeModel`, `OnActivate`, `OnToggle`; add matching `PropValue` variants: `CommandSet(CommandSet)`, `TableModelProp(TableModel)`, `TreeModelProp(TreeModel)` (rename to avoid clash with the type)
- [X] T012 Add `PropValue::reset_value` arms in `rn/src/reconciler.rs` (or wherever `prop_reset_value` is defined) for `CommandSet`, `TableModelProp`, `TreeModelProp` — returning an empty default of each
- [X] T013 Add `ComponentKind::Table` and `ComponentKind::Tree` to `ComponentKind::ALL` and increment `ComponentKind::COUNT` by 2 in `rn-core/src/lib.rs`
- [X] T014 Extend `assert_all_kinds_handled` in `rn-windows/src/renderer.rs` with arms for `ComponentKind::Table` and `ComponentKind::Tree`
- [X] T015 Extend `assert_all_kinds_handled` in `rn-linux/src/renderer.rs` with arms for `ComponentKind::Table` and `ComponentKind::Tree`
- [X] T016 Add `NativeHandle::TableWidget(Retained<NSScrollView>)` and `NativeHandle::TreeWidget(Retained<NSScrollView>)` variants to the `NativeHandle` enum in `rn-macos/src/renderer.rs`; add `as_view()` arms returning `Some(v)` for each; add zero-height `build_spec` stubs: `ComponentKind::Table | ComponentKind::Tree => (None, Some(0.0))`; add match arms in the `create_node` match that panic with `"Table/Tree not yet implemented"` (replaced in T028/T036) — the workspace must compile
- [X] T017 Add storybook catalogue entries (placeholder demos) for `ComponentKind::Table` and `ComponentKind::Tree` in `examples/storybook/src/catalog.rs` so `catalog_covers_every_component_kind` passes — demos can render a `label("Table — coming soon")` until T032 and T040 replace them

**Checkpoint**: `cargo clippy --workspace --all-targets -- -D warnings` exits 0; `cargo test --workspace --features rn/macos` exits 0; `cargo test -p storybook --features macos` passes the drift test; `RUSTFLAGS="-D warnings" cargo check -p rn-windows --target x86_64-pc-windows-msvc` exits 0 without errors. If this checkpoint fails, no user story work begins.

---

## Phase 3: User Story 1 — Command and keyboard layer (Priority: P1)

**Goal**: Applications can declare named commands that appear in the menu bar and dispatch via keyboard shortcut, with first-responder priority and availability rules.

**Independent Test**: `cargo run -p table-demo --features macos`, open its menu, invoke a shortcut while focused in a text field. Full validation in quickstart.md §A1.

- [X] T018 [US1] Implement `build_menu(command_set: &CommandSet, mtm: MainThreadMarker) -> Retained<NSMenu>` in `rn-macos/src/commands.rs` — iterates commands grouped by `placement.menu`, creates an `NSMenu` per group, creates `NSMenuItem` per command with `setTitle`, `setAction(sel!(fire:))`, `setTarget`, and `setKeyEquivalent` + `setKeyEquivalentModifierMask` from `KeyBinding`; items with `AvailabilityRule::Never` set `setEnabled(false)`; items with `WhenFlag` store the flag id for later evaluation; returns the top-level `NSMenu` with a leading app-name submenu as macOS requires
- [X] T019 [US1] Implement `CommandTarget` delegate class in `rn-macos/src/commands.rs` using `declare_class!` — ivars hold `handler: HandlerId`, `source: NodeId`; `#[method(fire:)]` dispatches via `rn_core::dispatch_event`; a `CommandSet::build_targets` function allocates one `CommandTarget` per command and returns them alongside the menu, keeping them alive (stash in `DelegateHandle::Commands(Vec<Retained<CommandTarget>>)`)
- [X] T020 [US1] Add `DelegateHandle::Commands(Vec<Retained<CommandTarget>>)` variant to the `DelegateHandle` enum in `rn-macos/src/renderer.rs`; stash from the Window create arm
- [X] T021 [US1] Add `PropKey::Commands` handling to the `ComponentKind::Window` create arm in `rn-macos/src/renderer.rs`: when props contain `PropValue::CommandSet(cs)`, call `build_menu` and `build_targets`, then `unsafe { NSApp::sharedApplication(mtm).setMainMenu(Some(&menu)) }` after the window is created; stash targets with `stash_delegate`
- [X] T022 [US1] Add `commands()` method to `ComponentBuilder` in `rn/src/component.rs`: `pub fn commands(self, cs: CommandSet) -> Self` inserting `PropKey::Commands => PropValue::CommandSet(cs)`; add `CommandBuilder`, `impl CommandBuilder { pub fn shortcut(…) pub fn available_when(…) pub fn in_menu(…) pub fn build(self) -> Command }` and `pub fn command(id: &str, title: &str, handler: HandlerId) -> CommandBuilder` in `rn/src/lib.rs`
- [X] T023 [US1] Update the `table-demo` stub in `examples/table-demo/src/main.rs` to declare three commands (one always-available, one with a WhenFlag, one with shortcut) on its window; the demo only needs to print the command id to prove dispatch
- [X] T024 [US1] Update storybook catalogue entry for Window in `examples/storybook/src/catalog.rs` to demonstrate a command set; run `cargo run -p table-demo --features macos` and follow quickstart.md §A1 to confirm menu, shortcut, unavailable state and first-responder priority

**Checkpoint**: The menu bar is populated from declared commands, shortcuts dispatch to the correct handler, a WhenFlag command is visibly disabled, and a shortcut pressed inside a focused field is handled by the field. `cargo test --workspace --features rn/macos` still passes.

---

## Phase 4: User Story 2 — Table of records with columns (Priority: P1)

**Goal**: Applications render a sortable, resizable, selectable table driven by `TableModel` data; 100,000 rows scroll within the render budget.

**Independent Test**: `cargo run -p table-demo --features macos -- --rows 100000`. Full validation in quickstart.md §A2 + §A3.

- [X] T025 [P] [US2] Implement `TableDataSource` in `rn-macos/src/table.rs` using `declare_class!` — ivars hold `model: TableModel`; implements `NSTableViewDataSource` with `numberOfRowsInTableView:` returning `model.rows.len()` and `tableView:sortDescriptorsDidChange:` firing a sort event via `dispatch_event`; implements `NSObjectProtocol`
- [X] T026 [P] [US2] Implement `TableDelegate` in `rn-macos/src/table.rs` using `declare_class!` — ivars hold shared model reference; implements `NSTableViewDelegate` with `tableView:viewForTableColumn:row:` returning a cell view built from the row's `TableCell` (label + optional image); implements `tableViewSelectionDidChange:` reading `selectedRow()` and dispatching `OnSelect` with the `RowId`; implements `tableView:sortDescriptorsDidChange:` — note: on first click sets ascending, on second click descending (alternates)
- [X] T027 [US2] Add `impl TableDataSource { pub fn new(mtm, model, source) -> Retained<Self> }` and `impl TableDelegate { pub fn new(mtm, model, source) -> Retained<Self> }` to `rn-macos/src/table.rs`; add `TableSources { data: Retained<TableDataSource>, delegate: Retained<TableDelegate> }` and `DelegateHandle::Table(TableSources)` variant to the `DelegateHandle` enum in `rn-macos/src/renderer.rs`
- [X] T028 [US2] Implement `ComponentKind::Table` create arm in `rn-macos/src/renderer.rs`: if props contain `PropValue::TableModelProp(model)`, create `NSScrollView` + `NSTableView`; add one `NSTableColumn` per `TableColumn` (setTitle, setWidth, setMinWidth, setResizingMask AutoresizingMask, setHeaderView from model); set `setAllowsColumnResizing(true)`, `setAllowsColumnSelection(false)`, `setSortDescriptors` for sortable columns; attach data source and delegate; `setDocumentView(outline)`; insert `NativeHandle::TableWidget(scroll)` and stash `DelegateHandle::Table`; update `build_spec` to give the Table component a fixed `height: 300.0` default (overridable by `PropKey::Height`) and `flex_grow: 1.0`
- [X] T029 [US2] Implement `update_node` arm for `(NativeHandle::TableWidget(sv), PropKey::TableModel)` in `rn-macos/src/renderer.rs` — update the data source's model, preserve selection for RowIds that still exist, call `reloadData()` on the `NSTableView`
- [X] T030 [US2] Add `pub fn table(model: TableModel) -> ComponentBuilder` builder in `rn/src/lib.rs`; add `table_column`, `table_row`, `table_cell`, `table_cell_icon` helpers returning their respective model types
- [X] T031 [US2] Implement `examples/table-demo/src/main.rs` fully — a window with a 5-column `TableModel` over up to `--rows N` generated rows (default 1000, `--rows 100000` for the scale test), sortable columns, row selection updating a status label; the commands from T023 remain
- [X] T032 [US2] Replace the placeholder storybook entry for `ComponentKind::Table` in `examples/storybook/src/catalog.rs` with a real demo: a 3-column table over 50 rows with a sortable column and row selection event
- [X] T033 [US2] Run `cargo run -p table-demo --features macos` and follow quickstart.md §A2 (sort, resize, selection, dark mode parity) then §A3 (`--rows 100000` scroll and memory); confirm `cargo test --workspace --features rn/macos` still passes

**Checkpoint**: A 5-column table renders, columns resize and sort, row selection fires an event, 100,000 rows scroll without stutter. The table looks like the platform's own table in light and dark mode.

---

## Phase 5: User Story 3 — Tree of nested items (Priority: P1)

**Goal**: Applications render an expandable outline whose expansion state survives a full data refresh keyed by node identity.

**Independent Test**: `cargo run -p tree-demo --features macos` (or the tree tab in table-demo). Full validation in quickstart.md §A4 + §A5.

- [X] T034 [P] [US3] Implement `TreeSource` in `rn-macos/src/tree.rs` using `declare_class!` — ivars hold `roots: Vec<TreeNode>`; implements `NSObjectProtocol`, `NSControlTextEditingDelegate`, `NSOutlineViewDataSource`, `NSOutlineViewDelegate` following the pattern already established for `SidebarModel` in `rn-macos/src/renderer.rs`; items are `NSNumber` tags encoding depth and index; `isGroupItem:` returns false for all items; `isItemExpandable:` returns true when the node has children; `viewForTableColumn:item:` returns a cell view with icon (if present) and label; `outlineViewSelectionDidChange:` dispatches `OnSelect` with the `RowId`; `outlineViewItemDidExpand:` / `outlineViewItemDidCollapse:` dispatch `OnToggle` with the new state
- [X] T035 [US3] Implement expansion-state restoration in `rn-macos/src/tree.rs` — `TreeSource::restore_expansion(outline: &NSOutlineView, ids: &[RowId])` iterates the outline rows after `reloadData()`, finds rows whose node id is in `ids`, and calls `expandItem:` on each; this is how FR-016 and the "survives refresh" scenario are satisfied
- [ ] T036 [US3] Implement `ComponentKind::Tree` create arm in `rn-macos/src/renderer.rs`: create `NSScrollView` + `NSOutlineView`; set `setStyle(NSTableViewStyle::SourceList)` only if the tree is inside a sidebar (determined by parent kind); otherwise use `NSTableViewStyle::Automatic`; `setHeaderView(None)`, `setIndentationPerLevel(16.0)`, add one auto-resizing column, `setOutlineTableColumn`; attach `TreeSource`; stash `DelegateHandle::Tree(Retained<TreeSource>)`; add `DelegateHandle::Tree` variant; update `build_spec` to give the Tree component `height: 300.0` default and `flex_grow: 1.0`; insert `NativeHandle::TreeWidget(scroll)` variant (mirror of `TableWidget`)
- [ ] T037 [US3] Implement `update_node` arm for `(NativeHandle::TreeWidget(sv), PropKey::TreeModel)` in `rn-macos/src/renderer.rs` — capture current expanded ids from the model before update, update the tree source's roots, call `reloadData()`, call `restore_expansion`, update selection; `expandItem_expandChildren(None, true)` is NOT called (that would ignore stored state)
- [ ] T038 [US3] Add `pub fn tree(model: TreeModel) -> ComponentBuilder` and `pub fn tree_node(id: RowId, label: &str) -> TreeNodeBuilder` in `rn/src/lib.rs`; `impl TreeNodeBuilder { pub fn icon, pub fn child, pub fn expanded, pub fn build }`
- [ ] T039 [US3] Add a tree tab or second window to `examples/table-demo/src/main.rs` (rename example to `data-demo` if desired) — renders a 3-level tree of 20 nodes, a refresh button that replaces the model with equivalent data, and a selection status label
- [ ] T040 [US3] Replace the placeholder storybook entry for `ComponentKind::Tree` in `examples/storybook/src/catalog.rs` with a real demo showing a two-level tree with expand/collapse and selection
- [ ] T041 [US3] Run the tree demo and follow quickstart.md §A4 (expand/collapse, selection, refresh preserves state) then §A5 (keyboard-only navigation — arrows, Enter); confirm `cargo test --workspace --features rn/macos` still passes

**Checkpoint**: A tree renders with disclosure affordances, expanding and collapsing works, a full model refresh preserves expansion and selection, keyboard navigation moves the selection and expands/collapses.

---

## Phase 6: Polish and Integration

**Purpose**: CI gate, component docs, commit. No story label.

- [ ] T042 Run the full Phase A CI gate in order: `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --features rn/macos`; `cargo test -p storybook --features macos`; `RUSTFLAGS="-D warnings" cargo check -p rn-windows --target x86_64-pc-windows-msvc`; report any failure and fix before proceeding
- [ ] T043 [P] Regenerate component docs via `scripts/gen-docs.sh` to capture Table and Tree screenshots in `docs/components/`
- [ ] T044 [P] Commit all planning artifacts (spec, plan, research, data-model, contracts, quickstart, checklists, AGENTS.md update, feature.json) and all implementation commits; `git push origin main`

**Checkpoint (Phase A exit)**: All four CI commands pass; `docs/components/` has Table and Tree entries; everything is committed and pushed. The menu bar works, a 100,000-row table scrolls in budget, and a tree preserves expansion across refresh.

---

## Dependencies and Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies — begin immediately
- **Phase 2 (Foundational)**: Depends on Phase 1 — **BLOCKS all user story work**
- **Phase 3 (US1)**: Depends on Phase 2 — no dependency on US2 or US3
- **Phase 4 (US2)**: Depends on Phase 2 — no dependency on US1 or US3
- **Phase 5 (US3)**: Depends on Phase 2 — no dependency on US1 or US2
- **Phase 6 (Polish)**: Depends on Phases 3, 4, and 5 all complete

### User Story Dependencies

- **US1** can begin immediately after Phase 2 — commands live in `commands.rs`, no file overlap with table or tree
- **US2** can begin immediately after Phase 2 — table lives in `table.rs`
- **US3** can begin immediately after Phase 2 — tree lives in `tree.rs`
- US1, US2 and US3 each add one arm to `renderer.rs`; that is the only shared file and the arms are independent match cases

### Within Each User Story

- Data types before renderer arm (T008-T010 before T028/T036)
- Renderer arm before builder (T028 before T030; T036 before T038)
- Builder before demo (T030 before T031; T038 before T039)
- Demo before storybook entry (demos prove the builder before the catalogue)
- Storybook entry before checkpoint validation

### Parallel Opportunities

- T003, T004, T005 run in parallel (different new files)
- T008, T009, T010 run in parallel (different data source files)
- T014, T015 run in parallel (different stub backend files)
- T025, T026 run in parallel (independent delegate classes in `table.rs`)
- T034 runs in parallel with T025/T026 (different file)
- US1 (T018–T024), US2 (T025–T033), US3 (T034–T041) can run in parallel after Phase 2
- T043, T044 run in parallel (different concerns)

---

## Parallel Example: US2 and US3 after Phase 2

```text
# Once T007–T017 are done, these can proceed in parallel:

US2 track:
  T025 TableDataSource + T026 TableDelegate (parallel)
  T027 constructors + DelegateHandle variant
  T028 ComponentKind::Table render arm
  T029 update_node arm
  T030 table() builder
  T031 table-demo

US3 track:
  T034 TreeSource declare_class
  T035 restore_expansion helper
  T036 ComponentKind::Tree render arm
  T037 update_node arm
  T038 tree() builder
  T039 tree in demo
```

---

## Implementation Strategy

### MVP First (US2 only — quickest visible win)

1. Phase 1: Setup → Phase 2: Foundational → Phase 4: US2 (table)
2. **STOP and VALIDATE**: 5-column table with 1,000 rows sorts, selects and resizes correctly
3. Extend with 100,000 rows to confirm the scale story

### Recommended Order (P1 first)

1. Phase 1 → Phase 2
2. Phase 3 (US1 commands) — proves dispatch and menu before data surfaces
3. Phase 4 (US2 table) — the largest Phase A deliverable
4. Phase 5 (US3 tree) — shares the pull-model pattern established by the sidebar
5. Phase 6 (polish and push)

### Phase B and C

Phase B (overlays, editor) and Phase C (forms, motion, test harness) are not in this
file. They share no foundational tasks with Phase A and get their own tasks file after
Phase A is committed and the gate is green.

---

## Notes

- [P] tasks have different file targets and no incomplete dependencies — safe to run in parallel
- `DelegateHandle` is a thread-local map; stashing keeps Retained objects alive past the render pass
- The `SidebarModel` in `renderer.rs` is the precedent for the pull-model data source pattern; read it before T025/T034
- `ComponentKind::COUNT` must equal `ComponentKind::ALL.len()` or the guard test fails — T013 updates both atomically
- The `completeness-guard` CI job injects `ZzzGuardVariant` and expects a non-exhaustive error; T014 and T015 extend the match rather than adding a catch-all
- `catalog_covers_every_component_kind` in the storybook test fails if any kind has no catalogue entry; T017 adds placeholders before implementation so the test never goes red during development
