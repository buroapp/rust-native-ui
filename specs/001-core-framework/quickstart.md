# Quickstart Validation Guide: Core Framework

This guide describes how to validate that the `rn` core framework is working
correctly end-to-end. It covers the two validation scenarios (Hello World and
Counter) that map directly to user stories P1 and P2.

---

## Prerequisites

- Rust toolchain (stable, 1.75+)
- macOS 13+, Windows 10 22H2+, or Linux with GTK 4.8+
- On Linux: `libgtk-4-dev` installed (`apt install libgtk-4-dev` or equivalent)

---

## Validation 1 — Hello World (proves P1: write-once cross-platform)

**Goal**: A window with a label and a button renders using native OS controls.
The exact same source compiles on all three platforms.

### Setup

```bash
cd examples/hello-world
cargo run --features rn/<platform>   # platform = macos | windows | linux
```

### Expected Outcome

- A native window opens with the title "Hello, Rust Native".
- A label reading "Hello, World!" is visible inside the window.
- A button labelled "Click me" is visible below the label.
- All controls are verified as native widgets (see verification steps below).
- Closing the window exits the process cleanly (exit code 0).

### Platform Verification

**macOS**: Open Accessibility Inspector (`Xcode → Open Developer Tool`). Inspect
the running app. Each control should show role `AXWindow`, `AXStaticText`,
`AXButton` — AppKit-native roles.

**Windows**: Open Accessibility Insights for Windows. Inspect the running process.
Controls should show `ControlType.Window`, `ControlType.Text`, `ControlType.Button`.

**Linux**: Run `accerciser` (GNOME accessibility explorer). The app should appear
in the tree with GTK4 widget roles.

---

## Validation 2 — Counter (proves P2: minimal reconciler updates)

**Goal**: Incrementing a counter updates only the label text. No other controls
are recreated.

### Setup

```bash
cd examples/counter
cargo run --features rn/<platform>
```

### Expected Outcome

- A window with a label showing "Count: 0" and an "Increment" button opens.
- Clicking "Increment" updates the label to "Count: 1", "Count: 2", etc.
- The button and window are not recreated between increments (no flicker).

### Reconciler Verification (headless, no OS required)

```bash
cargo test -p rn --test reconciler
```

All tests must pass. Key tests to look for:

| Test | Validates |
|------|-----------|
| `test_no_change_produces_empty_changeset` | SC-002 (no spurious updates) |
| `test_prop_change_produces_update_only` | P2 acceptance scenario 1 |
| `test_node_removal_produces_delete` | P2 acceptance scenario 2 |
| `test_no_platform_types_in_changeset` | P3 / FR-007 |

---

## Validation 3 — API isolation (proves P3: no platform type leakage)

**Goal**: A consumer crate using only `rn` as a UI dependency has zero platform
backend symbols in scope.

### Setup

```bash
cd examples/hello-world
cargo tree -p hello-world --edges normal
```

### Expected Outcome

- `objc2`, `windows`, and `gtk4` do NOT appear in the dependency tree for the
  consumer crate itself.
- They appear only as transitive deps of `rn-macos` / `rn-windows` / `rn-linux`
  which are internal to the framework workspace.

---

## Validation 4 — Memory stability (proves SC-006)

```bash
cargo test -p rn --test reconciler -- memory_stability --nocapture
```

Expected output: "1000 cycles complete. Heap delta: 0 bytes."

---

## References

- Public API contract: `contracts/public-api.md`
- Data model: `data-model.md`
- Research decisions: `research.md`
- Constitution: `.specify/memory/constitution.md`
