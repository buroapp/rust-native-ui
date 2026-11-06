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
- [X] T036 [US3] Implement `ComponentKind::Tree` create arm in `rn-macos/src/renderer.rs`: create `NSScrollView` + `NSOutlineView`; set `setStyle(NSTableViewStyle::SourceList)` only if the tree is inside a sidebar (determined by parent kind); otherwise use `NSTableViewStyle::Automatic`; `setHeaderView(None)`, `setIndentationPerLevel(16.0)`, add one auto-resizing column, `setOutlineTableColumn`; attach `TreeSource`; stash `DelegateHandle::Tree(Retained<TreeSource>)`; add `DelegateHandle::Tree` variant; update `build_spec` to give the Tree component `height: 300.0` default and `flex_grow: 1.0`; insert `NativeHandle::TreeWidget(scroll)` variant (mirror of `TableWidget`)
- [X] T037 [US3] Implement `update_node` arm for `(NativeHandle::TreeWidget(sv), PropKey::TreeModel)` in `rn-macos/src/renderer.rs` — capture current expanded ids from the model before update, update the tree source's roots, call `reloadData()`, call `restore_expansion`, update selection; `expandItem_expandChildren(None, true)` is NOT called (that would ignore stored state)
- [X] T038 [US3] Add `pub fn tree(model: TreeModel) -> ComponentBuilder` and `pub fn tree_node(id: RowId, label: &str) -> TreeNodeBuilder` in `rn/src/lib.rs`; `impl TreeNodeBuilder { pub fn icon, pub fn child, pub fn expanded, pub fn build }`
- [X] T039 [US3] Add a tree tab or second window to `examples/table-demo/src/main.rs` (rename example to `data-demo` if desired) — renders a 3-level tree of 20 nodes, a refresh button that replaces the model with equivalent data, and a selection status label
- [X] T040 [US3] Replace the placeholder storybook entry for `ComponentKind::Tree` in `examples/storybook/src/catalog.rs` with a real demo showing a two-level tree with expand/collapse and selection
- [X] T041 [US3] Run the tree demo and follow quickstart.md §A4 (expand/collapse, selection, refresh preserves state) then §A5 (keyboard-only navigation — arrows, Enter); confirm `cargo test --workspace --features rn/macos` still passes

**Checkpoint**: A tree renders with disclosure affordances, expanding and collapsing works, a full model refresh preserves expansion and selection, keyboard navigation moves the selection and expands/collapses.

---

## Phase 6: Polish and Integration

**Purpose**: CI gate, component docs, commit. No story label.

- [X] T042 Run the full Phase A CI gate in order: `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --features rn/macos`; `cargo test -p storybook --features macos`; `RUSTFLAGS="-D warnings" cargo check -p rn-windows --target x86_64-pc-windows-msvc`; report any failure and fix before proceeding
- [X] T043 [P] Regenerate component docs via `scripts/gen-docs.sh` to capture Table and Tree screenshots in `docs/components/`
- [X] T044 [P] Commit all planning artifacts (spec, plan, research, data-model, contracts, quickstart, checklists, AGENTS.md update, feature.json) and all implementation commits; `git push origin main`

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

## Phase 7: Convergence

**Purpose**: Close gaps identified by `/speckit-converge`. Phase A tasks were marked done but several implementation obligations were missing or partial.

- [X] T045 [US2] Implement `update_node` arm for `(NativeHandle::TableWidget, PropKey::TableModelKey)` in `rn-macos/src/renderer.rs` — retrieve `DelegateHandle::Table(sources)`, call `update_table_model(sources, new_model)`, call `selection.filter_to(&row_ids)` to drop removed rows, call `reloadData()` on the embedded `NSTableView` per T029 (missing)
- [X] T046 [US3] Implement `update_node` arm for `(NativeHandle::TreeWidget, PropKey::TreeModelKey)` in `rn-macos/src/renderer.rs` — capture expanded ids from `sources.inner.model`, call `update_tree_model(sources, new_model)`, call `reloadData()`, call `restore_expansion(ov, &expanded_ids)` per T037 (missing)
- [X] T047 [US3] Implement `restore_expansion(ov: &NSOutlineView, ids: &[RowId])` body in `rn-macos/src/tree.rs` — iterate all outline rows, decode each item to a FlatNode RowId, call `expandItem:` when id is in `ids`; call from T046 arm per FR-016 (missing)
- [X] T048 [US3] Wire `outlineViewItemDidExpand:` and `outlineViewItemDidCollapse:` in `TreeDelegate` in `rn-macos/src/tree.rs` — decode item to RowId, update the FlatNode `expanded` flag in the snapshot so model stays in sync with user interaction per FR-016 (missing)
- [X] T049 [US2/US3] Wire cell icon display in `make_cell_view` (`table.rs`) and `make_tree_cell` (`tree.rs`) — when `cell.icon`/`node.icon` is Some, fetch `NSImage::imageWithSystemSymbolName`, create `NSImageView`, add beside `NSTextField` in the `NSTableCellView` per FR-011 (missing)
- [X] T050 [US2] Implement table empty-state: when `model.rows.is_empty()` in the Table `create_node` arm, show a centred `NSTextField` label (default text 'No items', overridable by `PropKey::EmptyText`) overlaid on or above the scroll view per FR-014 (missing)
- [X] T051 [US2] Add `--rows N` argument to `examples/table-demo/src/main.rs` using `std::env::args()` — generate N rows with deterministic fake data (default 1000); add a status label that updates with the selected row Name per T031 scope (partial)
- [X] T052 [US3] Add a Refresh button and selection label to the tree pane in `examples/table-demo/src/main.rs` — Refresh replaces the model with same ids and new labels, proving expansion state survives; label shows selected node label per T039 scope (missing)
- [X] T053 [US1] Implement `AvailabilityRule::WhenFlag` menu validation: add `NSMenuDelegate::menuWillOpen:` on the bar (or `validateMenuItem:` on `ButtonTarget`) that re-evaluates each item's enabled state from current application flags per FR-003/FR-004 (missing)
- [X] T054 [US1] Add duplicate-shortcut detection in `install_main_menu_with_targets` — scan commands for duplicate `(key, modifiers)` pairs before installing and emit a `ShortcutConflict` warning per FR-006 (missing)
- [X] T055 [US2/US3] Wire `OnActivate` dispatch — set `doubleAction`/`action` selector or implement `keyDown` forwarding on `NSTableView`/`NSOutlineView` to dispatch `OnActivate` with the selected `RowId` on Enter or double-click per FR-018 (missing)
- [X] T056 [US2] Surface sort indicator: after updating `model.sort` in `sort_changed`, call `tv.setHighlightedTableColumn(Some(&col))` so the platform sort arrow appears per FR-010/SC-006 (partial)
- [X] T057 [US2] Add `multi_select(bool)` builder to `ComponentBuilder` and wire `PropKey::MultiSelect` to `setAllowsMultipleSelection` in the Table create arm; update `selectionDidChange` to collect all selected indices per FR-009 (missing)
- [X] T058 Replace `docs/components/img/TableTree.png` with a clean isolated window screenshot (no browser background) per T043 scope (partial)

## Phase 8: Convergence

**Purpose**: Close gaps found by second `/speckit-converge` pass. All Phase 7 tasks are done; these are the remaining Phase A spec obligations.

- [X] T059 [US2] Apply sort to row data in `sort_changed` in `rn-macos/src/table.rs` — after determining `new_sort`, stable-sort `model.rows` in-place by the named column's cell text (ascending/descending); call `reloadData()` after; sort must be stable so equal-text rows keep a consistent order per spec edge case per FR-010/US2/AC2 (missing)
- [X] T060 [US2] Fix multi-select `selectionDidChange` in `rn-macos/src/table.rs` — replace `tv.selectedRow()` with `tv.selectedRowIndexes()`, iterate all set bits, collect corresponding `RowId`s, dispatch `OnSelect` with all selected ids; single-select path remains unchanged per FR-009/US2/AC3 (missing)
- [X] T061 [US2] Manage empty-state label across live refreshes in the `update_node` Table arm in `rn-macos/src/renderer.rs` — when `new_model.rows.is_empty()` add the label to the scroll view; when non-empty remove any existing empty-state subview; the label must appear/disappear correctly as data changes per FR-014 (missing)
- [X] T062 [US2] Set `NSTableColumn` resizing mask in the Table `create_node` arm in `rn-macos/src/renderer.rs` — call `setResizingMask(NSTableColumnResizingOptions::NSTableColumnUserResizingMask)` on each column so columns are interactively resizable by dragging the divider per FR-008/US2/AC1 (missing)
- [X] T063 [US2] Preserve scroll position on table model refresh in `update_node` Table arm in `rn-macos/src/renderer.rs` — before `reloadData()`, read the first visible row index from the scroll view's `documentVisibleRect`; after `reloadData()`, call `scrollRowToVisible:` clamped to the new row count per FR-012/edge (missing)
- [X] T064 [US1] Implement `WhenFlag` dynamic availability: add a `FLAG_REGISTRY: HashMap<FlagId, bool>` thread_local in `rn-macos/src/commands.rs`; add `pub fn set_command_flag(id: FlagId, value: bool)` callable from application code; add `validateMenuItem:` on `ButtonTarget` that returns `NO` when `AvailabilityRule::WhenFlag(id)` maps to false per FR-003/FR-004 (missing)
- [X] T065 [US2/US3] Wire `OnActivate` dispatch on Enter/double-click: use `NSTableView::setAction(Some(sel!(fire:)))` + `setTarget` (not `doubleAction`) for table; for tree add `outlineViewShouldSelectItem:` returning false is wrong — instead set the table's `action` to a `CommandTarget`-style object that dispatches `handler_activate`; suppress this if `handler_activate` is None per FR-018/US2/US3 (missing)
- [X] T066 [US2] Add `--rows 100000` validation note to `docs/components/Table.md` and `specs/005-native-app-foundation/quickstart.md` — document that `cargo run -p table-demo --features macos -- --rows 100000` validates SC-003 (smooth scroll) and SC-004 (memory note: 100k TableRows are heap-allocated; NSTableView view-virtualization satisfies per-frame budget) per SC-003/SC-004 (partial)
- [X] T067 [US2/US3] Verify `NSTableColumn::setTitle` propagates as AX column header name — launch table-demo, run `xcrun accessibilityinspector` (or AX CLI) and confirm column headers are named; if not, call `col.setAccessibilityTitle` explicitly in the Table create arm per FR-019/SC-008 (partial)

## Phase 9: Convergence

**Purpose**: Close gaps found by third `/speckit-converge` pass. All Phase 8 tasks are done.

- [X] T068 [US2] Fix sort indicator in `sort_changed` in `rn-macos/src/table.rs` — the `setHighlightedTableColumn` block currently reads `model.sort` (old state) instead of `new_sort` (just computed); move the highlighted-column update to read `new_sort` so the indicator correctly reflects the column that was just sorted on first click and on column switches per FR-010/US2/AC2 (partial)
- [X] T069 [US2] Update `model.selection.selected` in `selectionDidChange` in `rn-macos/src/table.rs` — after collecting `selected_ids`, borrow `inner.model` mutably and set `model.selection.selected = selected_ids.clone(); model.selection.anchor = selected_ids.first().copied()`; this makes `filter_to` effective and keeps the in-memory selection consistent with the NSTableView per FR-009/US2/AC3 (missing)
- [X] T070 [US3] Wire `OnActivate` dispatch for tree: add an `activate` field to `TreeSources` in `rn-macos/src/tree.rs`, wire `NSOutlineView::setDoubleAction(Some(sel!(fire:)))` + `setTarget` in the Tree `create_node` arm in `rn-macos/src/renderer.rs`, mirroring the table's pattern per FR-018/US3 (missing)
- [X] T071 [US2] Fix icon+text layout in `make_cell_view` in `rn-macos/src/table.rs` — `text_x` is computed but discarded (`let _ = text_x`); create the `NSTextField` with an x-origin offset equal to `text_x` (20pt when icon present, 2pt otherwise) so the label is positioned to the right of the icon and they do not overlap per FR-011/US2 (partial)
- [X] T072 [US1] Update `NSMenuItem` enabled state on flag change — implement `NSMenuDelegate::menuWillOpen:` on the menu bar: iterate menu items, for each one whose `ButtonTarget` has `WhenFlag(id)`, call `item.setEnabled(flag_is_enabled(id))`; register the delegate on the bar after `setMainMenu` per FR-003/FR-004 (missing)
- [X] T073 [US2] Preserve scroll position on column sort: in `sort_changed` in `rn-macos/src/table.rs`, before calling `tv.reloadData()`, capture `tv.rowAtPoint(tv.visibleRect().origin)` as the first visible row; after `reloadData()`, call `tv.scrollRowToVisible(first_visible_row.min(rows_count - 1))` so the user's scroll position is not reset on sort per FR-010/edge (partial)

## Phase 10: Convergence

**Purpose**: Close final MEDIUM/LOW gaps found by fourth `/speckit-converge` pass. All HIGH findings from prior passes are resolved.

- [X] T074 [US3] Write `inner.model.borrow_mut().selection = Some(row_id)` in `selection_changed` in `rn-macos/src/tree.rs` — tree node selection is dispatched as an event but never stored back in `TreeModel.selection`; this matches the table fix in T069 and makes the model's selection consistent with the NSOutlineView per FR-016/US3 (missing)
- [X] T075 [US1] Change `install_main_menu_with_targets` in `rn-macos/src/commands.rs` to return shortcut conflicts to the caller instead of only logging them — define a `pub struct ShortcutConflict { a: String, b: String }` in `rn-core/src/data/command.rs`, return `Vec<ShortcutConflict>` from the function, and surface it through the `commands()` builder or a separate `check_shortcuts` helper so application code can inspect conflicts per FR-006/SC-001 (partial)
- [X] T076 [US2] Enable horizontal scrolling on the table scroll container in `rn-macos/src/renderer.rs` — currently `new_scroll_container` calls `setHasHorizontalScroller(false)` for all scroll views including the table; for the table, call `sv.setHasHorizontalScroller(true)` after `new_scroll_container` returns so the surface scrolls horizontally when the window is narrower than the combined minimum column widths per edge/spec (partial)
- [X] T077 [US3] Cap tree indentation at a maximum depth in `rn-macos/src/renderer.rs` — `setIndentationPerLevel(16.0)` gives unbounded growth; after building the FlatNode snapshot, compute the maximum depth in the tree and if it exceeds 6 levels, set `setIndentationPerLevel(96.0 / max_depth as f64)` so the deepest label still receives at least 2pt of indentation clearance from its parent's disclosure affordance per edge/FR-015 (partial)

## Phase 11: Convergence

**Purpose**: Close final HIGH findings from fifth `/speckit-converge` pass. Visual selection is not restored after data refresh on either table or tree.

- [ ] T078 [US2] Restore NSTableView visual selection after `reloadData()` in the `update_node` Table arm in `rn-macos/src/renderer.rs` — after `reloadData()` and scroll restoration, iterate `sources.inner.model.borrow().selection.selected`, find each RowId's current position in the sorted row list (by scanning `model.rows` for matching id), build an `NSIndexSet` from those positions, and call `tv.selectRowIndexes_byExtendingSelection(&index_set, false)` so the visual selection matches the model after every data refresh per SC-006/FR-009/US2/AC3 (partial)
- [ ] T079 [US3] Restore NSOutlineView visual selection after `reloadData()` + `restore_expansion()` in the `update_node` Tree arm in `rn-macos/src/renderer.rs` — after restoration, read `sources.inner.model.borrow().selection`, find the outline row for that RowId by iterating `ov.numberOfRows()` and matching via `ov.itemAtRow()` → NSNumber → snapshot RowId, then call `ov.selectRowIndexes_byExtendingSelection(&set, false)` so the selected node remains highlighted across refreshes per SC-006/FR-016/US3/AC3 (partial)
