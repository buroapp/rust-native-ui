# Data Model: Native App Foundation

**Feature**: `005-native-app-foundation` | **Phase**: 1 | **Date**: 2026-11-05

All types below live in `rn-core` and are plain Rust: no platform types, no interior
mutability, no handles. The macOS crate converts them into native data sources.

---

## Table model (US2, FR-007 to FR-014)

### `TableColumn`

| Field | Type | Notes |
|---|---|---|
| `id` | `ColumnId` | Stable identity, used for sort descriptor and width state |
| `title` | `String` | Header text |
| `width` | `f32` | Current width in points |
| `min_width` | `f32` | Floor enforced on drag and window resize (FR-008, edge case: narrow window) |
| `alignment` | `TextAlign` | Left, center, right |
| `sortable` | `bool` | Whether the header responds to clicks (FR-010) |

**Validation**: `width >= min_width`, `min_width > 0`. A column whose width is below its
minimum is not constructible through the builder, and is clamped if it arrives that way.

### `TableCell`

| Field | Type | Notes |
|---|---|---|
| `text` | `String` | Display text |
| `icon` | `Option<String>` | Leading icon name (FR-011) |

**Validation**: none beyond non-empty `text` or present `icon`. A cell with neither
renders as empty rather than as an error.

### `TableRow`

| Field | Type | Notes |
|---|---|---|
| `id` | `RowId` | Application-supplied stable identity (FR-012) |
| `cells` | `Vec<TableCell>` | One per column, in column order |
| `selectable` | `bool` | Defaults to true |

**Validation**: `cells.len()` must equal the model's column count. Mismatched rows are
rejected at construction rather than rendering short.

### `TableModel`

| Field | Type | Notes |
|---|---|---|
| `columns` | `Vec<TableColumn>` | Ordered, at least one |
| `rows` | `Vec<TableRow>` | May be empty (FR-014 empty state) |
| `sort` | `Option<SortDescriptor>` | Absent when unsorted |
| `selection` | `Selection` | Current selection |

**Relationships**: `TableModel` owns its columns and rows. `RowId` is referenced by
`Selection` and by expansion state in the tree, never by index, so identity survives
refresh (FR-012, SC-006).

### `SortDescriptor`

| Field | Type | Notes |
|---|---|---|
| `column` | `ColumnId` | Which column is sorted |
| `direction` | `SortDirection` | `Ascending` or `Descending` |

**State transition**: unsorted to ascending on first header click; ascending to
descending on the next; descending back to ascending. There is no third "unsorted"
state, matching FR-010's alternating requirement. Equal values keep insertion order, so
repeated sorts are stable (edge case).

### `Selection`

| Field | Type | Notes |
|---|---|---|
| `selected` | `Vec<RowId>` | Ordered set; one element for single selection |
| `anchor` | `Option<RowId>` | Row keyboard movement continues from |

**State transitions**:
- Click with no modifier: selection becomes exactly that row, anchor becomes it.
- Click with modifier: added to or removed from selection, anchor becomes it.
- Arrow key: anchor moves to the adjacent row; selection becomes exactly the new anchor.
- Anchor row removed from the data: selection resolves to the neighbour, or clears if
  none remains (edge case, FR-012).

**Invariants**: every `RowId` in `selected` exists in the model's rows. A selection
referring to a vanished row is filtered when the data changes, before any event is sent.

---

## Tree model (US3, FR-015 to FR-017)

### `TreeNode`

| Field | Type | Notes |
|---|---|---|
| `id` | `RowId` | Shared identity type with the table, so both surfaces key state the same way |
| `label` | `String` | Row text |
| `icon` | `Option<String>` | Optional leading icon |
| `children` | `Vec<TreeNode>` | Empty for leaves |
| `expanded` | `bool` | Ignored for leaves |

**Validation**: depth is computed during a pre-order flatten, not stored per node, so a
node cannot carry a wrong depth. A cycle cannot be constructed because children are owned
by value.

**Expansion state**: `expanded` is carried by the model and re-applied by identity after
a refresh, which is what satisfies FR-016 and the "survives refresh" acceptance scenario.

### `TreeModel`

| Field | Type | Notes |
|---|---|---|
| `roots` | `Vec<TreeNode>` | Ordered |
| `selection` | `Option<RowId>` | Tree is single-select in this version |

**Flattening**: the native outline control asks for children per item, so no eager flatten
is required. Depth is supplied by the control's own indentation.

---

## Command model (US1, FR-001 to FR-006)

### `Command`

| Field | Type | Notes |
|---|---|---|
| `id` | `CommandId` | Stable identity |
| `title` | `String` | Menu item text |
| `shortcut` | `Option<KeyBinding>` | Optional |
| `available` | `AvailabilityRule` | Whether the command can run now |
| `handler` | `HandlerId` | Reuses the existing dispatch path (FR-002) |
| `placement` | `CommandPlacement` | Which menu it belongs to, and its order |

### `KeyBinding`

| Field | Type | Notes |
|---|---|---|
| `key` | `String` | Key identifier |
| `modifiers` | `Modifiers` | Bitset: command, shift, option, control |

**Validation**: registration can fail when the system or another command already owns the
combination. Failure is reported to the application, not swallowed (FR-006, edge case).

### `AvailabilityRule`

**Closed enum**, so an invalid rule cannot be constructed (Article III):

- `Always`
- `WhenFlag(FlagId)` for application-supplied state such as "has a selection"
- `Never` for temporarily withdrawn commands

**State transition**: evaluated when the menu is opened, so a change made while the menu is
closed is reflected next time (FR-004, edge case). A command evaluating to unavailable is
shown disabled and its shortcut does not dispatch (FR-003).

---

## Overlay model (US4, FR-020 to FR-024)

### `Overlay`

| Field | Type | Notes |
|---|---|---|
| `anchor` | `NodeId` | The control it attaches to |
| `placement` | `OverlayPlacement` | `Below`, `Above`, `Leading`, `Trailing`, or `Automatic` |
| `dismiss` | `DismissPolicy` | Outside click, Escape, action chosen, or all |
| `traps_focus` | `bool` | True for decision-demanding popovers (FR-023) |

**State transitions**: closed to open on activation; open to closed on any policy trigger,
on anchor removal, or when the anchor scrolls out of view (FR-021, edge case). Closing
always returns focus to the anchor.

### `MenuItem`

| Field | Type | Notes |
|---|---|---|
| `label` | `String` | |
| `handler` | `HandlerId` | |
| `shortcut` | `Option<KeyBinding>` | |
| `enabled` | `bool` | |
| `separator_before` | `bool` | Visual grouping |

---

## Document model (US5, FR-025 to FR-028)

### `Document`

| Field | Type | Notes |
|---|---|---|
| `text` | `String` | Whole content |
| `selection` | `Option<TextRange>` | Anchor and head offsets |
| `undo_depth` | `usize` | Bound on the native undo stack |

**Validation**: selection ranges are always within `text.len()` and are clamped when text
changes make them invalid. Undo stops at the document as opened, so undoing past the start
does not empty the document (edge case).

**State transitions**: each edit pushes an undo step; undo moves back one step; redo moves
forward until the newest edit, then becomes unavailable.

---

## Form model (US6, FR-029 to FR-031)

### `FormField`

| Field | Type | Notes |
|---|---|---|
| `id` | `FieldId` | Key in the submitted result (FR-030) |
| `label` | `String` | |
| `value` | `FieldValue` | `Text(String)` or `Number(f64)` |
| `rules` | `Vec<ValidationRule>` | All must pass |
| `touched` | `bool` | Gates whether a message may show (FR-031) |

**State transitions**: untouched to touched on first edit or on submit attempt. Becoming
invalid from outside data (for example a rule change) shows the message even when
untouched, which is the documented exception to FR-031.

### `ValidationRule`

**Closed enum**:

- `Required`
- `MinLength(usize)`
- `MaxLength(usize)`
- `Numeric`
- `Range { min: f64, max: f64 }`
- `Matches(Pattern)` where `Pattern` is a constrained form, not free-form code

Each rule carries its message, so an invalid field has text ready and no message
construction happens in the render path (Article IV).

### `FormState`

| Field | Type | Notes |
|---|---|---|
| `fields` | `Vec<FormField>` | |
| `submit_attempted` | `bool` | |

**Invariants**: `is_valid()` is derived from the fields, never stored, so a stale valid
flag cannot exist. Submission is refused while any rule fails (FR-029).

---

## Motion and test models (US7, US8)

### `MotionPreference`

`Reduced` or `Full`. Read from the platform at transition time, not cached, so a change
mid-session is honoured (edge case).

### `TestSession`

| Field | Type | Notes |
|---|---|---|
| `tree` | rendered component tree | Production path, not a simulation |
| `layout` | computed rects | Assertable by node |
| `accessibility` | role, name, state per element | Requires the offscreen native hierarchy |

**Invariants**: a session always runs the production reconciler and layout. Assertions read
state, layout and accessibility; none of them read internal renderer fields that production
does not use.

---

## Entity relationships

```text
TableModel 1 ── n TableColumn
TableModel 1 ── n TableRow ── n TableCell
TableModel 1 ── 0..1 SortDescriptor ── 1 ColumnId
TableModel 1 ── 1 Selection ── n RowId

TreeModel  1 ── n TreeNode (recursive, owned by value)
TreeModel  1 ── 0..1 RowId (selection)         # shares the identity type with the table

CommandSet 1 ── n Command ── 0..1 KeyBinding
Command    1 ── 1 AvailabilityRule ── 0..n FlagId

Overlay    1 ── 1 NodeId (anchor)
Overlay    1 ── 1 OverlayPlacement
Overlay    1 ── 1 DismissPolicy

FormState  1 ── n FormField ── n ValidationRule
FormState  1 ── 0..1 FormResult                # produced only when valid

Document   1 ── 0..1 TextRange
```

## How the model maps to component kinds

| Component kind | Model it owns | Native control it drives |
|---|---|---|
| `Table` | `TableModel` | platform table |
| `Tree` | `TreeModel` | platform outline |
| `Editor` | `Document` | platform text view |
| `Form` | `FormState` | platform views per field, no native container |
| `FormField` | `FormField` | platform text field |
| `Window` (extended) | `CommandSet` via a new prop | application main menu |

No new kind is introduced for columns, rows, cells or tree nodes: those are data, which is
the whole point of this design.
