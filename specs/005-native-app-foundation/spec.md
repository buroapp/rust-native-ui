# Feature Specification: Native App Foundation

**Feature Branch**: `005-native-app-foundation`

**Created**: 2026-11-05

**Status**: Draft

**Input**: User description: "Catalogue comparison against gpui-kit identified five missing capability areas: a command and keyboard layer, data surfaces, anchored overlays, rich text editing, and application plumbing (forms, motion, UI test harness). Build all five."

## Scope map

This feature covers five capability areas. They expand into eight independently
testable user stories because three of the areas (data surfaces, and application
plumbing) contain parts that deliver value on their own.

| Area | Stories |
|---|---|
| Command and keyboard layer | US1 |
| Data surfaces | US2, US3 |
| Anchored overlays | US4 |
| Rich text editing | US5 |
| Application plumbing (forms, motion, testing) | US6, US7, US8 |

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Command and keyboard layer (Priority: P1)

An application developer declares named commands, each with a title, an optional keyboard shortcut, an optional availability rule, and a handler. The application's menu bar lists those commands. Pressing the shortcut runs the command against whatever has focus. Commands that are not currently applicable appear unavailable and cannot be invoked. Commands that a focused text input would normally handle (for example typing shortcuts inside a text field) are left to that field.

**Why this priority**: Professional desktop software is driven by menus and shortcuts. Without this, no application built on the framework can offer a working menu bar or a single keyboard shortcut, which makes every other feature harder to use at speed. It is the largest conceptual gap identified in the comparison.

**Independent Test**: Declare three commands with three shortcuts, run the application, and confirm each shortcut reaches its handler, that a command marked unavailable is greyed in the menu and does nothing when its shortcut is pressed, and that a shortcut pressed while a text field is focused is handled by the field instead of the command.

**Acceptance Scenarios**:

1. **Given** an application with two commands bound to different shortcuts, **When** the user presses one shortcut, **Then** exactly one handler runs and it is the matching command.
2. **Given** a command whose availability rule is currently false, **When** the user opens its menu, **Then** the item is shown unavailable and cannot be chosen; pressing its shortcut does nothing.
3. **Given** focus is inside a text input, **When** the user presses a shortcut that the text input itself uses, **Then** the text input handles it and the application command does not run.
4. **Given** an application is frontmost, **When** the user chooses a command from the menu bar, **Then** the same handler runs as would run from the shortcut.
5. **Given** a command's availability rule changes while the menu is closed, **When** the user next opens the menu, **Then** the item reflects the new availability without restarting.

---

### User Story 2 - Table of records with columns (Priority: P1)

An application developer has a set of records and needs to present them as a table with named columns. They describe the columns once and supply row data, and the application renders a table that looks and behaves like the platform's own: resizable columns, a header row, row selection, and sorting by clicking a header. Selecting a row reports the selection to the application.

**Why this priority**: This is the most requested capability for information-dense desktop software and the most conspicuous gap against comparable frameworks. Queue monitors, transaction lists and audit views cannot be built today, because a flat list has no columns, headers or sortable presentation.

**Independent Test**: Render five named columns over 1,000 records, click a column header and confirm rows reorder with a visible sort indicator, then select a row and confirm the application receives the event. Delivers a shippable table screen with no dependency on other stories.

**Acceptance Scenarios**:

1. **Given** a table is displayed, **When** the user drags a column divider, **Then** that column resizes and the neighbouring column absorbs the change.
2. **Given** a sortable column, **When** the user clicks its header once, **Then** rows sort ascending with an ascending indicator; **When** clicked again, **Then** rows sort descending and the indicator reflects that.
3. **Given** no row is selected, **When** the user clicks a row, **Then** that row shows the platform's selection treatment and the application receives a selection event naming the row.
4. **Given** more rows than fit on screen, **When** the user scrolls to the end, **Then** the last row is visible and all rows remain addressable in order.
5. **Given** a table with zero rows, **When** the screen renders, **Then** a readable empty message is shown instead of a blank surface.

---

### User Story 3 - Tree of nested items (Priority: P1)

An application developer has hierarchical data and needs an expandable outline. Parent rows show a disclosure affordance, expanding reveals children indented by depth, collapsing hides them, and the expanded state survives data refreshes so the tree does not collapse under the user while they work.

**Why this priority**: Hierarchy is the second shape information-dense software needs. Without it, applications flatten their navigation, which loses meaning.

**Independent Test**: Render a three-level tree of 50 nodes, expand a branch, collapse it, refresh the data, and confirm expansion state survived. Delivers a usable outline screen independently of the table.

**Acceptance Scenarios**:

1. **Given** a collapsed parent, **When** the user activates its disclosure affordance, **Then** children appear directly beneath it, indented one level, and the affordance indicates expanded.
2. **Given** an expanded parent whose child is selected, **When** the user collapses the parent, **Then** the row disappears and the selection resolves to a defined, non-crashing state.
3. **Given** a tree whose data is replaced with equivalent data, **When** the refresh completes, **Then** expanded branches stay expanded and the selected row stays selected.
4. **Given** a row at depth three, **When** it renders, **Then** its indentation visibly exceeds a depth-one row and its label does not collide with the disclosure affordance.

---

### User Story 4 - Anchored overlays (Priority: P2)

An application developer attaches a popover, a tooltip, or a menu to a specific control. The overlay appears adjacent to that control, stays anchored while the window scrolls or resizes, and dismisses on an outside click, on Escape, or when its action is chosen. A popover that demands a decision keeps keyboard focus inside itself until it closes.

**Why this priority**: The framework already has a popover that cannot anchor to anything and a tooltip that never appears on hover, so both are non-functional placeholders. Anchoring is what turns them into usable controls, and three components are fixed by one mechanism.

**Independent Test**: Anchor a popover to a button near the bottom-right of the window, open it, confirm it is placed without leaving the window, tab through its contents without escaping, then press Escape and confirm focus returns to the button.

**Acceptance Scenarios**:

1. **Given** a control with an attached popover, **When** the user activates the control, **Then** the popover appears adjacent to it fully inside the window.
2. **Given** an open popover containing several controls, **When** the user presses Tab repeatedly, **Then** focus cycles within the popover and never reaches controls behind it.
3. **Given** an open popover, **When** the user presses Escape or clicks outside it, **Then** it closes and focus returns to the control that opened it.
4. **Given** a control with a tooltip, **When** the pointer rests over it, **Then** the tooltip appears after a short delay and disappears when the pointer leaves.
5. **Given** an open overlay, **When** the user scrolls or resizes the window, **Then** the overlay stays attached to its control and repositions to remain visible.

---

### User Story 5 - Rich text editing (Priority: P2)

An application developer needs an editable multi-line text area where the user can select, replace, copy, paste, undo and redo using the platform's standard editing keys, with the caret and selection behaving as they do in any native text field.

**Why this priority**: The framework can display styled text but cannot edit text beyond a basic single-purpose field. Any application that takes notes, edits a template, or holds a comment needs this before it can be considered usable for real work.

**Independent Test**: Open an editor preloaded with text, select a range with the keyboard, type over it, undo, redo, and copy the selection to the clipboard, verifying the text content after each step.

**Acceptance Scenarios**:

1. **Given** an editor containing text, **When** the user holds Shift and presses an arrow key, **Then** a selection extends character by character and is visibly highlighted.
2. **Given** a selection, **When** the user types a character, **Then** the selection is replaced and the text content reported to the application reflects it.
3. **Given** the user has made an edit, **When** they invoke undo and then redo, **Then** the text returns to the pre-edit state and then to the edited state.
4. **Given** a selection, **When** the user copies and pastes into another application, **Then** the clipboard contains the selected text.
5. **Given** the editor has focus, **When** the user presses a shortcut claimed by an application command, **Then** text editing keys still behave as the platform expects.

---

### User Story 6 - Forms with validation (Priority: P3)

An application developer describes a form as a set of fields with rules, and the application displays inline messages for invalid fields while preventing submission until the form is valid. The submitted values are delivered as a structured result the application reads by field.

**Why this priority**: Every settings screen, dialog and onboarding flow needs it, but it can be assembled today from raw inputs at the cost of writing validation by hand each time. It is a multiplier rather than a blocker.

**Independent Test**: Build a form with a required field and a numeric field, attempt submission while empty, correct each field, and confirm submission only succeeds once both are valid and the delivered values match what was typed.

**Acceptance Scenarios**:

1. **Given** a form with a required field, **When** the user submits while it is empty, **Then** submission is refused and an inline message explains the requirement.
2. **Given** an invalid field, **When** the user corrects it, **Then** the message clears without submitting.
3. **Given** a form where every rule passes, **When** the user submits, **Then** the application receives each field value keyed by field.
4. **Given** a field the user has not yet visited, **When** the form first renders, **Then** no validation message appears until the field is touched or the form is submitted.

---

### User Story 7 - Motion that respects user settings (Priority: P3)

An application developer asks for a transition, and it plays as a smooth animation on a normal machine, but becomes instantaneous when the user has asked the operating system to reduce motion.

**Why this priority**: The framework has no animation at all, which is why several surfaces feel abrupt, but motion must never override an accessibility setting. Shipping this after the core surfaces is correct; shipping it without the preference is not acceptable.

**Independent Test**: Trigger the same transition twice, once with reduce-motion enabled at the operating system level and once without, and confirm duration is near zero in the first case and visible in the second, with identical end state.

**Acceptance Scenarios**:

1. **Given** reduce-motion is enabled, **When** a transition runs, **Then** the change is applied without a visible intermediate state.
2. **Given** reduce-motion is disabled, **When** a transition runs, **Then** a smooth intermediate state is visible.
3. **Given** either preference, **When** a transition completes, **Then** the resulting state is identical.

---

### User Story 8 - Automated interface testing (Priority: P3)

An application developer writes a test that renders a component tree without showing a window, drives input against it, and asserts on the resulting state, layout and accessibility output, so regressions are caught before release.

**Why this priority**: The framework has unit tests for layout and engines but no way to test a screen. As component count grows, manual screenshots stop being sufficient, so this raises the ceiling on everything else.

**Independent Test**: Write one test that renders a screen, clicks a control by accessibility label, and asserts the resulting state change, running without a visible window.

**Acceptance Scenarios**:

1. **Given** a test that renders a component tree, **When** the test runs, **Then** no window appears and the tree's layout is computed.
2. **Given** a rendered tree, **When** the test sends a click to a control found by its accessibility label, **Then** the corresponding handler runs.
3. **Given** a rendered tree, **When** the test asserts on a row's accessibility role and name, **Then** the assertion reports pass or fail with the observed value.

---

### Edge Cases

- A command is bound to a shortcut that the operating system already owns: the application is told the binding could not be registered rather than silently failing.
- Two commands claim the same shortcut: the conflict is detected and reported, not resolved by load order.
- A table's data set is replaced while scrolled: scroll position and selection stay meaningful, or clamp to a valid row, never point past the end.
- The selected row is removed by an update: selection moves to a defined neighbour or clears, and no event names a row that no longer exists.
- A column is dragged narrower than its content: text truncates with an ellipsis rather than overflowing.
- The window is resized narrower than the combined minimum column widths: columns hold their minimums and the surface scrolls horizontally.
- Sorting on a column with duplicate values: equal rows keep a stable order across repeated sorts.
- A tree is deeper than the indentation allowance: depth stops increasing the inset and labels remain readable.
- A tooltip's control is removed from the screen while the tooltip is showing: the tooltip disappears rather than floating orphaned.
- A popover's control scrolls out of view while the popover is open: the popover dismisses rather than detaching.
- The user undoes past the point where the document was opened: undo stops at the original document rather than emptying it.
- A form field becomes invalid because data changed outside the form: the message appears without the user having touched the field.
- Reduce-motion is toggled while the application is running: the next transition honours the new setting without a restart.
- A test drives input at a control that is disabled: the input is ignored and the assertion reports no state change.

## Requirements *(mandatory)*

### Functional Requirements

**Command and keyboard layer**

- **FR-001**: Applications MUST be able to declare named commands, each carrying a title, an optional keyboard shortcut, an optional availability rule, and a handler.
- **FR-002**: The application's menu bar MUST reflect declared commands, and choosing a menu item MUST run the same handler as the equivalent shortcut.
- **FR-003**: A command whose availability rule is false MUST appear unavailable and MUST NOT run when its shortcut is pressed.
- **FR-004**: Availability changes MUST be reflected the next time the menu is opened, without restarting the application.
- **FR-005**: Keyboard shortcuts MUST dispatch to the focused context first, so a focused text input that handles a shortcut prevents an application command from stealing it.
- **FR-006**: Attempting to bind a shortcut that the operating system or another command already owns MUST be reported to the application rather than failing silently.

**Data surfaces**

- **FR-007**: The table MUST render rows from application-supplied data against named columns, without the application constructing a component per row.
- **FR-008**: Columns MUST have a title, a current width and a minimum width, and MUST be resizable by dragging the divider between headers.
- **FR-009**: The table MUST support single and multiple row selection and MUST report selection changes through the existing event mechanism.
- **FR-010**: Clicking a sortable column header MUST sort by that column, alternating direction on repeated clicks, and MUST display the active direction.
- **FR-011**: Cells MUST support plain text and an optional leading icon.
- **FR-012**: Rows MUST carry a stable application-supplied identity so a refresh preserves selection, sort order and scroll position.
- **FR-013**: The table MUST materialize only rows in the visible region plus a small buffer, so scrolling cost does not grow with total row count.
- **FR-014**: The table MUST show a readable empty state when it has no rows.
- **FR-015**: The tree MUST render hierarchical data supplied by the application and MUST express depth through indentation.
- **FR-016**: Tree rows MUST expand and collapse, and expanded state MUST be preserved across refreshes keyed by row identity.
- **FR-017**: The tree MUST show a disclosure affordance on rows with children and MUST NOT show one on leaves.
- **FR-018**: Both surfaces MUST support keyboard operation: arrows move selection and scroll it into view, left and right collapse and expand in the tree, and Enter or Space activates the focused row.
- **FR-019**: Both surfaces MUST expose accessibility semantics for row content, column headers, selection state and expansion state.

**Anchored overlays**

- **FR-020**: Popovers, tooltips and menus MUST attach to a specific control and appear adjacent to it, positioned to remain fully inside the window.
- **FR-021**: An open overlay MUST stay attached to its control when the window scrolls or resizes, and MUST dismiss if the control leaves the visible area.
- **FR-022**: An overlay MUST close on an outside click, on Escape, and on choosing one of its actions.
- **FR-023**: A popover that demands a decision MUST keep keyboard focus inside itself until it closes, and MUST return focus to its control afterwards.
- **FR-024**: A tooltip MUST appear after a short hover delay and disappear when the pointer leaves or the control is removed.

**Rich text editing**

- **FR-025**: The editor MUST support multi-line editing with a caret and a character selection operable from the keyboard.
- **FR-026**: The editor MUST support the platform's standard editing operations: insert, replace selection, delete, copy, paste, undo and redo.
- **FR-027**: The editor MUST report its text content to the application after each change, and MUST accept content supplied by the application.
- **FR-028**: The editor MUST leave shortcuts it does not use to the command layer rather than consuming all key input.

**Application plumbing**

- **FR-029**: Forms MUST be declarable as fields with rules, MUST display inline messages for invalid fields, and MUST refuse submission while any rule fails.
- **FR-030**: A submitted form MUST deliver its values keyed by field.
- **FR-031**: A field MUST NOT show a validation message before it is touched or the form is submitted, except when data changed outside the form made it invalid.
- **FR-032**: Transitions MUST be visually smooth by default and MUST become instantaneous when the operating system reports that motion should be reduced, with the same end state in both cases.
- **FR-033**: A component tree MUST be renderable without a visible window, and MUST accept synthetic pointer and keyboard input for testing.
- **FR-034**: Tests MUST be able to assert on resulting state, computed layout and accessibility role, name and state of any element.
- **FR-035**: Both data surfaces MUST be driven by the platform's native data control, so selection treatment, header behaviour, focus ring, dark mode, accent colour and text size follow the operating system rather than being approximated.
- **FR-036**: Row height, header metrics and overlay spacing MUST derive from platform values rather than hand-picked constants.
- **FR-037**: Adding all of the above MUST NOT change the behaviour of any existing component, and the existing component catalogue MUST remain complete.

### Key Entities *(include if feature involves data)*

- **Command**: a named application action. Carries an identity, a title, an optional shortcut, an optional availability rule, and a handler.
- **Keybinding**: a shortcut attached to a command, which may fail to register because the system or another command owns it.
- **Focus Context**: the innermost surface that currently receives keyboard input, used to decide whether a shortcut belongs to a control or to a command.
- **Column**: a named presentation axis of a table. Carries a title, current and minimum width, an alignment, and whether it is sortable.
- **Row**: one record in a table. Carries a stable identity, a value per column, and whether it is selectable.
- **Row Identity**: an application-supplied key that binds selection, sort and expansion state to a record rather than a position.
- **Node**: one entry in a tree. Carries an identity, a label, an optional icon, a depth, an expanded state, and ordered children.
- **Selection**: the set of selected row identities plus the anchor row that keyboard movement continues from.
- **Sort Descriptor**: the column currently sorted and its direction, absent when unsorted.
- **Anchor**: the control an overlay is attached to, used for placement and lifetime.
- **Overlay**: a popover, tooltip or menu with an anchor, a dismissal policy, and an optional focus trap.
- **Document**: the editable text content of an editor, with a caret position and zero or more selection ranges, plus an undo history.
- **Field**: one input in a form. Carries an identity, a value, the rules that apply, and whether it has been touched.
- **Validation Rule**: a predicate over a field value with a message shown when it fails.
- **Motion Preference**: whether the operating system asks for reduced motion.
- **Test Session**: a render of a component tree without a window, plus the input and assertions applied to it.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every declared command is reachable from both the menu bar and its shortcut, and a shortcut conflict is reported in 100% of conflict cases.
- **SC-002**: A shortcut pressed inside a focused text input is handled by that input in 100% of cases where the input owns the key.
- **SC-003**: Scrolling a 100,000-row table remains smooth throughout, with no frame exceeding the platform's smooth-animation budget on minimum supported hardware.
- **SC-004**: Resident memory for a 100,000-row table differs from a 1,000-row table by bookkeeping only, and does not grow in proportion to row content.
- **SC-005**: A developer can render a sortable five-column table over 10,000 records in under 30 lines of application code.
- **SC-006**: A data refresh that replaces the whole data set preserves selection and expansion state for every identity that still exists, in 100% of cases.
- **SC-007**: 100% of table, tree and overlay operations are reachable using only the keyboard.
- **SC-008**: Accessibility inspection reports zero unnamed rows and zero unnamed column headers, and exposes selected and expanded state on every applicable row.
- **SC-009**: An overlay never renders outside the window bounds and never remains visible after its anchor leaves the visible area, across 100% of scroll and resize cases tested.
- **SC-010**: Every standard editing operation (insert, replace, delete, copy, paste, undo, redo) produces the expected text content in 100% of test cases.
- **SC-011**: A form refuses submission in 100% of invalid states and delivers every field value on success.
- **SC-012**: Transitions complete with the same end state with and without reduced motion, and are visually instantaneous when the preference is set.
- **SC-013**: A screen can be tested end to end without a visible window, including input dispatch and accessibility assertions.
- **SC-014**: The existing component catalogue still reports full coverage, all existing demos render unchanged, and no previously passing test regresses.
- **SC-015**: Side-by-side comparison with the operating system's own table, outline, menu and text controls shows matching selection treatment, focus ring and header behaviour, with no visually distinct hand-drawn substitute.

## Assumptions

- This feature supersedes the earlier table-and-tree-only specification, which is folded in as US2 and US3. That draft directory is replaced rather than kept alongside.
- Application developers supply data, row identities and form rules; the framework owns presentation, interaction and validation dispatch, not fetching or persistence.
- macOS is the only target that becomes functional now. Windows and Linux receive the same component surface but remain explicit not-implemented stubs, consistent with the paused platform-parity work.
- In-place cell editing is out of scope; the data surfaces are for display, selection and navigation in this version.
- The tree is single-column in this version. A tree with multiple columns is a follow-on because it multiplies both the data contract and the native mapping.
- Row heights are uniform within a surface. Variable-height rows are a follow-on.
- The editor is plain text with formatting-free content. Syntax highlighting, language services and rich text styling are follow-ons.
- Menus are application-level menus derived from commands; context menus attached to a control are a follow-on that reuses the same anchoring mechanism.
- Commands dispatch to the focused context through the platform's existing key handling rather than a new global keyboard hook.
- Grouping and aggregation rows, charts, diff views, dockable panels and a scripting extension surface were reviewed in the same catalogue comparison and deliberately deferred.
- Undo history is per document and in-memory; it does not persist across application launches.
- The test harness drives the same rendering path as production rather than a separate simulation, so a passing test exercises real layout.
