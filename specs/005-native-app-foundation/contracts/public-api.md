# Public API Contract: Native App Foundation

**Feature**: `005-native-app-foundation` | **Phase**: 1 | **Date**: 2026-11-05

Additive only. Nothing in this contract changes an existing signature, so the change is
semver-minor for the `rn` crate. No type in this file is a platform type (Article III).

---

## 1. Commands (US1)

```rust
// rn
pub fn command(id: &str, title: &str, handler: HandlerId) -> CommandBuilder;

impl CommandBuilder {
    pub fn shortcut(self, key: &str, modifiers: Modifiers) -> Self;
    pub fn available_when(self, flag: FlagId) -> Self;
    pub fn always_available(self) -> Self;
    pub fn in_menu(self, menu: &str, order: u32) -> Self;
    pub fn build(self) -> Command;
}

// Attach to a window; the window installs the application's main menu.
ComponentBuilder::commands(self, commands: impl Into<CommandSet>) -> Self;
```

**Contract**:
- Every declared command appears in the menu and is reachable by its shortcut, or a
  conflict is reported through a registration error (FR-001, FR-002, FR-006).
- `available_when` is evaluated when the menu opens (FR-004). An unavailable command is
  disabled and its shortcut does not dispatch (FR-003).
- Shortcut dispatch defers to the focused control first (FR-005).

**Events**: commands reuse `HandlerId` and the existing dispatch path. No new event type.

---

## 2. Table (US2)

```rust
// rn
pub fn table(model: TableModel) -> ComponentBuilder;

pub fn table_column(id: &str, title: &str, width: f32) -> TableColumnBuilder;
impl TableColumnBuilder {
    pub fn min_width(self, min: f32) -> Self;
    pub fn align(self, align: TextAlign) -> Self;
    pub fn sortable(self) -> Self;
    pub fn build(self) -> TableColumn;
}

pub fn table_row(id: RowId) -> TableRowBuilder;
impl TableRowBuilder {
    pub fn cell(self, text: impl Into<String>) -> Self;
    pub fn cell_icon(self, text: impl Into<String>, icon: &str) -> Self;
    pub fn selectable(self, selectable: bool) -> Self;
    pub fn build(self) -> TableRow;
}
```

**Props**: `PropKey::TableModel` carrying `PropValue::TableModel(TableModel)`,
`PropKey::OnSelect`, `PropKey::OnActivate`.

**Contract**:
- Rows come from the model, never from builder children (FR-007).
- Columns resize by drag and honour `min_width` (FR-008).
- Single and multiple selection are both supported and reported on change (FR-009).
- Header click alternates ascending and descending and displays the direction (FR-010).
- Cells render text plus an optional leading icon (FR-011).
- A row with an identity that persists keeps its selection across refresh (FR-012).
- Only visible rows are materialized (FR-013).
- An empty model renders the empty state, not a blank surface (FR-014).

**Events**:
- `OnSelect` with a payload naming the selected `RowId` values.
- `OnActivate` with a payload naming the activated `RowId`.

**Errors**: constructing a `TableRow` whose cell count differs from the model's column
count returns a construction error rather than rendering a short row.

---

## 3. Tree (US3)

```rust
// rn
pub fn tree(model: TreeModel) -> ComponentBuilder;

pub fn tree_node(id: RowId, label: &str) -> TreeNodeBuilder;
impl TreeNodeBuilder {
    pub fn icon(self, icon: &str) -> Self;
    pub fn child(self, node: TreeNode) -> Self;
    pub fn expanded(self, expanded: bool) -> Self;
    pub fn build(self) -> TreeNode;
}
```

**Props**: `PropKey::TreeModel`, `PropKey::OnSelect`, `PropKey::OnToggle`.

**Contract**:
- Hierarchy comes from the model; depth is expressed by native indentation (FR-015).
- Expansion state is preserved across refresh keyed by node identity (FR-016).
- Disclosures appear only on nodes with children (FR-017).
- Collapsing a parent whose child is selected leaves a defined selection (edge case).

**Events**: `OnSelect` with a `RowId`; `OnToggle` with a `RowId` and the new expanded state.

---

## 4. Overlays (US4)

```rust
// rn
pub fn popover(anchor: NodeId) -> ComponentBuilder      // now functional
pub fn tooltip(anchor: NodeId, text: &str) -> ComponentBuilder
pub fn menu(anchor: NodeId, items: Vec<MenuItem>) -> ComponentBuilder
```

**Props**: `PropKey::Anchor` carrying `PropValue::NodeRef(NodeId)`,
`PropKey::Placement`, `PropKey::DismissPolicy`, `PropKey::TrapsFocus`.

**Contract**:
- Overlays appear adjacent to their anchor and fully inside the window (FR-020).
- They follow or dismiss when the anchor scrolls or resizes (FR-021).
- They close on outside click, Escape, and action choice (FR-022).
- A focus-trapping popover keeps focus inside and returns it on close (FR-023).
- Tooltips appear after a hover delay and vanish when the pointer leaves (FR-024).

**Backward compatibility**: `popover()` and `tooltip()` keep their existing signatures.
Their behaviour changes from placeholder to functional, which is a fix, not a break.

---

## 5. Editor (US5)

```rust
// rn
pub fn editor(text: impl Into<String>) -> ComponentBuilder;
```

**Props**: `PropKey::Text`, `PropKey::OnChange` with the new text as payload,
`PropKey::Editable`.

**Contract**:
- Multi-line editing with a caret and keyboard selection (FR-025).
- Insert, replace, delete, copy, paste, undo, redo all behave as the platform expects
  (FR-026).
- Content is reported after each change and accepted from the application (FR-027).
- The editor does not consume keys it does not handle (FR-028).

---

## 6. Forms (US6)

```rust
// rn
pub fn form(fields: Vec<FormField>) -> ComponentBuilder;
pub fn form_field(id: &str, label: &str) -> FormFieldBuilder;
impl FormFieldBuilder {
    pub fn required(self) -> Self;
    pub fn min_length(self, len: usize) -> Self;
    pub fn max_length(self, len: usize) -> Self;
    pub fn numeric(self) -> Self;
    pub fn range(self, min: f64, max: f64) -> Self;
    pub fn build(self) -> FormField;
}
```

**Props**: `PropKey::FormState`, `PropKey::OnSubmit` with a `FormResult` payload.

**Contract**:
- Invalid fields show inline messages and block submission (FR-029).
- A valid submission delivers every value keyed by field id (FR-030).
- No message appears before a field is touched or submission is attempted (FR-031).

**Rejected**: `ValidationRule::Matches` with arbitrary code. Rules are a closed enum, so no
application-supplied predicate executes inside the render path.

---

## 7. Motion (US7)

```rust
// rn
pub fn transition(duration_secs: f64) -> Transition;
```

**Contract**: applies the duration when motion is allowed and zero when the platform
reports reduced motion, with an identical end state (FR-032).

---

## 8. Testing (US8)

```rust
// rn::testing
pub struct TestSession { /* ... */ }

impl TestSession {
    pub fn render(tree: ComponentTree) -> Self;              // no visible window
    pub fn click_by_label(&mut self, label: &str) -> bool;
    pub fn key(&mut self, key: &str, modifiers: Modifiers) -> bool;
    pub fn rect_of(&self, node: NodeId) -> Option<Rect>;
    pub fn accessibility(&self, node: NodeId) -> Option<AccessibilityInfo>;
    pub fn asserts_ok(&self) -> bool;
}
```

**Contract**: the session drives the production reconciler and layout (FR-033), accepts
synthetic input (FR-033), and exposes state, layout and accessibility for assertions
(FR-034). Input at a disabled control is ignored and reports no state change (edge case).

---

## Compatibility and completeness obligations

Every new `ComponentKind` (`Table`, `Tree`, `Editor`, `Form`, `FormField`) must also be:

1. Listed in `ComponentKind::ALL` and reflected in `ComponentKind::COUNT`.
2. Given an arm in `rn-windows` and `rn-linux` `assert_all_kinds_handled`, or the
   `completeness-guard` CI job fails by design.
3. Given an entry in the storybook catalogue, or `catalog_covers_every_component_kind`
   fails.
4. Handled in the reconciler's `PropValue` reset path if it introduces a value variant.

`Window` gains a `Commands` prop; it is not a new kind.
