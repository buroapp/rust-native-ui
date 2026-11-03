# Quickstart Validation Guide: React Native Component Parity + Shadcn Design System

**Feature**: `002-react-native-parity` | **Phase**: 1

This guide validates that the feature is correctly implemented. Run these scenarios against a built binary to verify end-to-end behavior.

---

## Prerequisites

- Rust stable toolchain installed
- macOS 13+ (for macOS scenarios)
- Repo cloned: `~/Developer/rust-native-ui`

---

## Build

```bash
cd ~/Developer/rust-native-ui

# Build the public API crate (no OS required)
cargo build -p rn

# Build macOS backend
cargo build -p rn-macos --features macos

# Build settings demo example
cargo build -p settings-demo --features macos

# Build mail demo example
cargo build -p mail-demo --features macos
```

All commands must produce zero errors and zero warnings.

---

## Scenario 1: FlatList Virtualization (SC-001, SC-002)

**Purpose**: Confirm virtualized list with 10,000 items scrolls at 60fps with ≤5MB live memory.

```bash
cargo run -p settings-demo --features macos -- --bench-flatlist
```

**Expected**:
- Window opens with a `FlatList` of 10,000 items
- Scrolling at any speed is visually smooth (no visible jank)
- Console output shows: `live_nodes: N` where N ≤ 25 at all times
- Console output shows: `mem_rss_kb: N` where N ≤ 5120 (5MB)

**Manual verification**: Open Activity Monitor → select the demo process → confirm Real Memory stays below 5MB throughout scrolling.

---

## Scenario 2: Design System Tokens (SC-009)

**Purpose**: Confirm token presets render correctly on macOS native controls.

```bash
cargo run -p showcase --features macos -- --design-system
```

**Expected**:
- A window with four button variants side by side: `Primary`, `Secondary`, `Destructive`, `Ghost`
- Primary button uses the macOS accent color
- Destructive button shows red tint on text or border
- Ghost button is transparent with a subtle border
- Card component below the buttons has rounded corners, shadow, and a white/card background tint
- Badge shows color variants (default, secondary, destructive, success, warning) as small tinted pills
- Skeleton components pulse with a shimmer animation

---

## Scenario 3: macOS Settings App Layout (SC-007)

**Purpose**: Confirm Settings-style layout renders natively.

```bash
cargo run -p settings-demo --features macos
```

**Expected**:
- Window opens with a sidebar on the left containing 5–6 category items with SF Symbol icons
- Selecting a sidebar item highlights it and shows the corresponding settings panel on the right
- Right panel contains `GroupedSection` components with rounded corners and tinted background
- Each grouped section contains: `Switch` controls, `SegmentedControl`, `SearchBar` at the top
- Resizing the window narrows the sidebar below 160px, triggering collapse to icons-only
- `SearchBar` text input updates in real time (typed text appears immediately)
- `SegmentedControl` selection fires an event and the selected segment is highlighted

---

## Scenario 4: macOS Mail App Layout (SC-008)

**Purpose**: Confirm three-pane Mail-style layout with independent scrolling.

```bash
cargo run -p mail-demo --features macos
```

**Expected**:
- Window opens with three panes: sidebar (folder list), message list, message detail
- The divider between panes is draggable — resizing one pane expands the adjacent one
- Message list shows 100 items via `FlatList` and scrolls smoothly without jumping
- Message detail pane shows rich text with mixed bold/italic/colored spans using attributed strings
- Both the message list and message detail scroll **independently** — scrolling one pane does not affect the other
- `Toolbar` at the top of the detail pane shows action buttons (Reply, Forward, Delete) with SF Symbol icons
- Dragging the divider past the minimum pane width (160px) snaps it back smoothly

---

## Scenario 5: TouchableOpacity Event Bubbling (SC-005)

**Purpose**: Confirm press events bubble correctly and nested handlers all fire.

```bash
cargo run -p showcase --features macos -- --event-bubbling
```

**Expected**:
- Window shows three nested `TouchableOpacity` wrappers, each with a different label
- Clicking the innermost fires the inner handler AND the outer handlers in order (inner → outer)
- Console shows: `handler: inner`, `handler: middle`, `handler: outer` in that order
- Clicking an outermost region only fires the outer handler
- Visual opacity feedback: during press, inner wrapper dims to 0.4 opacity

---

## Scenario 6: SafeAreaView Insets (SC-006)

**Purpose**: Confirm content is inset correctly by macOS safe area.

```bash
cargo run -p showcase --features macos -- --safe-area
```

**Expected**:
- Window shows a `SafeAreaView` with a bright red border
- The red border sits below the macOS title bar (not overlapping it)
- The content inside the safe area starts below the traffic-light buttons row
- Console shows the computed insets: `top: 36px` (or platform-measured value)

---

## Scenario 7: Headless Reconciler Tests (SC-005)

**Purpose**: Confirm new component types reconcile correctly without a display server.

```bash
cargo test -p rn --tests
```

**Expected**: All tests pass including new scenarios for:
- `FlatList` virtualization state transitions (scroll → recycle → mount)
- `Modal` visibility state machine (Hidden → Presenting → Visible → Dismissing)
- `Style` token application (tokens produce expected `PropValue::Style` entries)
- `SegmentedControl` selection event round-trip
- `SplitView` pane resize event round-trip

---

## Failure Modes to Check

| Symptom | Likely Cause |
|---|---|
| FlatList shows blank rows during fast scroll | Buffer size too small — increase `buffer_rows` |
| Sidebar items don't highlight on selection | `selected` prop not being reconciled on re-render |
| Skeleton shimmer freezes | CALayer animation not restarting after view recycle |
| Modal appears behind main window | Modal not parented at NSWindow level |
| SplitView divider jumps when dragging | `min_pane_sizes` constraint not enforced in drag handler |
| TouchableOpacity opacity doesn't reset | Alpha not restored on `mouseUp`/release event |
| SearchBar shows no text | `value` prop not synced on first render |
