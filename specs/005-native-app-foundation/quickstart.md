# Quickstart: Native App Foundation

**Feature**: `005-native-app-foundation` | **Phase**: 1 | **Date**: 2026-11-05

Runnable validation scenarios that prove the feature works end to end. One section per
phase. Commands assume the repository root.

Contract details live in [contracts/public-api.md](./contracts/public-api.md). Types live
in [data-model.md](./data-model.md). This file is the run guide, not the implementation.

---

## Prerequisites

```sh
# Toolchain and workspace
rustc --version          # 1.75 or newer
cargo build --workspace  # baseline must be clean before any phase

# Baseline gate that must stay green throughout
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --features rn/macos
```

Expected: zero clippy errors, zero failing tests.

---

## Phase A: commands, table, tree

### A1. Menu bar and shortcuts

```sh
cargo run -p table-demo --features macos
```

Then verify by hand, because the menu bar is a native surface:

1. The application menu lists the declared commands.
2. Pressing a command's shortcut runs its handler (the demo prints the command id).
3. A command declared with `available_when` over state that is currently false appears
   disabled and its shortcut does nothing.
4. Click inside the demo's search field, press the shortcut that the field itself uses,
   and confirm the field handles it instead of the command.

Expected: all four behave as described. Item 4 is the one that proves FR-005 ordering.

### A2. Table correctness

```sh
cargo run -p table-demo --features macos
```

1. Click a column header: rows sort ascending, indicator shows ascending. Click again:
   descending, indicator follows.
2. Drag a divider between two headers: both columns resize, the frame holds at minimum
   widths, and the surface scrolls horizontally rather than collapsing columns.
3. Select a row: it shows the platform's selection treatment and the status line names
   the selected row id.
4. Confirm the table looks like the platform's own table, not a drawn imitation: focus
   ring, selection colour and header background match system appearance, including in
   dark mode.

### A3. Table scale

```sh
cargo run -p table-demo --features macos -- --rows 100000
```

1. Scroll continuously from top to bottom. No stutter, and rows stay in order.
2. Compare resident memory with `--rows 1000`: the difference should be bookkeeping only,
   not proportional to row content.
3. Sort the 100,000 rows: sorting is observable, and scrolling stays smooth afterwards.

Expected: this is the scenario that fails first if rows ever become component nodes.

### A4. Tree behaviour

```sh
cargo run -p tree-demo --features macos
```

1. Expand a collapsed parent: children appear beneath it, indented, disclosure flips.
2. Collapse a parent whose child is selected: no crash, selection resolves to a defined
   row.
3. Trigger the demo's refresh action, which replaces the model with equivalent data:
   expanded branches stay expanded and the selected row stays selected.

### A5. Keyboard-only pass

Without using the pointer:

1. Tab into the table, press Down repeatedly to the last row: selection advances and
   scrolls into view.
2. In the tree, press Right on a collapsed parent to expand, Left to collapse.
3. Press Enter on a focused row: activation fires.

Expected: everything in A2 and A4 is reachable from the keyboard alone.

**Phase A exits when** A1 through A5 pass and the baseline gate is still green.

---

## Phase B: overlays and editor

```sh
cargo run -p showcase --features macos -- --select Popover
cargo run -p showcase --features macos -- --select Editor
```

### B1. Anchored overlay

1. Activate a control near the bottom-right of the window: the popover appears adjacent
   and fully inside the window.
2. Tab repeatedly inside the popover: focus cycles and never escapes behind it.
3. Press Escape: it closes and focus returns to the control that opened it.
4. Scroll the anchor out of view while the popover is open: it dismisses rather than
   floating detached.
5. Hover a control with a tooltip: it appears after a delay and disappears when the
   pointer leaves.

### B2. Editor

1. Select a range with Shift plus arrows, type over it: selection is replaced and the
   reported text matches.
2. Undo then redo: the document returns to the pre-edit and edited states.
3. Copy a selection and paste into another application: clipboard content matches.
4. Press a shortcut owned by an application command while the editor has focus: text
   editing keys still behave as the platform expects.

**Phase B exits when** B1 and B2 pass, overlay accessibility is inspectable, and the
baseline gate is still green.

---

## Phase C: forms, motion, testing

```sh
cargo run -p showcase --features macos -- --select Form
```

### C1. Form validation

1. Submit an empty required field: submission refused, inline message shown.
2. Correct the field: message clears without submitting.
3. Submit a valid form: the application receives each value keyed by field id.
4. Confirm no message appears in a field the user has not touched and has not submitted.

### C2. Reduced motion

```sh
# Toggle the OS preference, then rerun the same transition
open "x-apple.systempreferences:com.apple.preference.universalaccess"
```

1. With reduce motion off: the transition is visibly smooth.
2. With reduce motion on: the same transition is instant.
3. In both cases the end state is identical.

Toggling the preference while the demo runs must affect the next transition without a
restart.

### C3. Headless test harness

```sh
cargo test -p rn --features testing
```

The suite must include at least:

1. A test that renders a tree with no visible window and asserts a computed layout rect.
2. A test that clicks a control by accessibility label and asserts the resulting state.
3. A test that asserts a row's accessibility role, name and selected state.
4. A test that sends input to a disabled control and asserts no state change.

Expected: all pass, and no window appears during the run.

**Phase C exits when** C1 through C3 pass and the baseline gate is still green.

---

## Full-feature validation

```sh
# Everything, in the same order CI runs it
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --features rn/macos
cargo test -p storybook --features macos          # catalogue drift test
cargo build -p rn-windows --target x86_64-pc-windows-msvc   # stub completeness

# Regenerate component docs once new kinds render
scripts/gen-docs.sh
```

Expected: clippy clean, all tests pass, the storybook catalogue covers every kind, the
stub backends still compile with an exhaustive match, and `docs/components/` gains entries
for the new kinds.

## Regression guard

The following must remain true after every phase, because they are the things this feature
is most likely to break:

- All 60 existing component kinds still render in storybook, unchanged.
- `catalog_covers_every_component_kind` passes.
- `completeness-guard` in CI still fails on an unhandled kind, which means the stub
  matches were extended rather than weakened with a catch-all.
- The sidebar, toolbar and split-view chrome from feature 004 is unchanged.
