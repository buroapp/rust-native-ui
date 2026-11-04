# Feature Specification: Native Chrome — Sidebar, Toolbar, Titlebar

**Feature Branch**: `004-native-chrome`

**Created**: 2026-11-04

**Status**: Draft

**Input**: User observation: "UI is native macOS components — we're not setting paddings and etc by ourselves?"

**Context**: The framework uses real AppKit *controls* — `NSButton`, `NSTextField`, `NSSwitch`, `NSTabView`, `NSAlert` — but rebuilds the *chrome* around them by hand. Sidebar rows are an `NSBox` plus a label plus an invisible button; the toolbar is a bare `NSView` with buttons placed at `x += 36.0`; the title strip is a hand-drawn view. The result looks close to macOS and does not behave like it: no selection animation, no icons, no row insets, no toolbar overflow, no user-resizable sidebar, and nothing changes when Apple changes the design language.

Flex layout — `padding`, `gap`, `flex`, `justify` — is correctly the framework's own concern and is out of scope here. This feature is about the chrome that AppKit already provides as first-class objects.

## Audit

Measured against the current macOS renderer:

| AppKit type | Used | Current hand-rolled substitute |
|---|---:|---|
| `NSToolbar` / `NSToolbarItem` | 0 | `NSView`, buttons at fixed `x` offsets, 36pt stride |
| `NSOutlineView` (source list) | 0 | `NSBox` + label + transparent `NSButton` per row |
| `NSSplitViewController` | 0 | plain `NSView` with flex panes |
| `NSTitlebarAccessoryViewController` | 0 | hand-drawn title strip view |
| `NSFont.systemFontSize` | 0 (now used) | literal `13.0` in 11 places |

## User Scenarios & Testing

### User Story 1 - Sidebar behaves like a real source list (Priority: P1)

A developer builds an application with a sidebar. It has icons, the selection highlight animates, rows have the standard inset and height, the sidebar can be collapsed with the standard toolbar button, its width can be dragged, and in dark mode it is vibrant without the application doing anything.

**Why this priority**: The sidebar is the single most visible chrome element and currently the largest imitation. It is also where the missing pieces hurt most: no icons, no animation, fixed width, and a hand-painted selection colour.

**Independent Test**: Launch the storybook. Click through every sidebar row and observe the selection highlight animate between rows rather than jump, with the row icon and text inset sitting on the standard grid. Drag the sidebar's trailing edge and observe it resize within the system's minimum and maximum.

**Acceptance Scenarios**:

1. **Given** a sidebar with rows, **When** the user selects a different row, **Then** the highlight transitions using the system selection behaviour rather than an instant fill change.
2. **Given** a sidebar row with an icon, **When** it renders, **Then** the icon and its text use the standard source-list insets and row height.
3. **Given** a window with a sidebar, **When** the user drags the divider, **Then** the sidebar resizes and remembers nothing custom — the width comes from the split view, not a magic number.
4. **Given** the application is switched to dark mode, **When** the sidebar redraws, **Then** the vibrancy and text colours are the system's, with no application colour choice involved.

---

### User Story 2 - Toolbar is a real NSToolbar (Priority: P1)

A developer declares toolbar actions. The toolbar appears in the window's titlebar area, items are spaced and sized by the system, extra items collapse into the standard overflow menu when the window is narrow, and the user can customise the toolbar if the application allows it.

**Why this priority**: The current toolbar computes item positions by incrementing an x offset, which means it cannot overflow, cannot be customised, and drifts from the system's spacing the moment that spacing changes.

**Independent Test**: Build a window with more toolbar items than fit at the minimum window width. Shrink the window and confirm the excess items move into the system overflow menu rather than being clipped off the edge.

**Acceptance Scenarios**:

1. **Given** a window with five toolbar items, **When** the window is narrowed below the combined width, **Then** the overflow control appears and the hidden items are reachable from it.
2. **Given** a toolbar item with a label, **When** the window is wide enough, **Then** the item shows its label using the system's display mode rather than a fixed size chosen by the application.
3. **Given** the user has not customised the toolbar, **When** the window opens, **Then** item order and spacing are the system defaults.

---

### User Story 3 - Title strip is the window's own titlebar (Priority: P2)

A developer asks for a titled window. The title, and any titlebar accessories, are drawn by the window system — including the modern look where content runs under a transparent titlebar.

**Why this priority**: Lower than the other two because it is the least interactive, but it is the reason the modern window look currently cannot be used: a hand-drawn title strip cannot cooperate with `FullSizeContentView`.

**Independent Test**: Open a window in modern look mode. The titlebar is transparent, content extends beneath it, and the application is told the top inset to keep its content clear of the window controls.

**Acceptance Scenarios**:

1. **Given** a window declared with the modern look, **When** it opens, **Then** the titlebar is transparent and the content area starts at the window's top edge.
2. **Given** such a window, **When** the application lays out its content, **Then** it can query the titlebar inset instead of guessing a padding value.

---

### Edge Cases

- A sidebar with zero rows must still show its vibrancy background and remain collapsible.
- A toolbar with a single item must not render an overflow control.
- A window whose sidebar is collapsed must give the freed width back to the content pane.
- A source-list row that is disabled must not be selectable.
- Rows taller than the standard height, if a caller asks for them, must not break the standard insets.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The sidebar MUST be built from a source-list-styled outline view hosted in a split view's sidebar item, not from a generic container with hand-positioned rows.
- **FR-002**: Sidebar rows MUST support an icon alongside their label.
- **FR-003**: Sidebar selection MUST use the system's selection appearance and animation.
- **FR-004**: Sidebar width MUST come from the split view, with user resizing and system minimum/maximum, and no hard-coded default in the renderer.
- **FR-005**: Selecting a sidebar row MUST deliver the same application event as today, so existing applications keep working unchanged.
- **FR-006**: The toolbar MUST be an `NSToolbar` with `NSToolbarItem`s built from the declared actions.
- **FR-007**: Toolbar overflow MUST be handled by the system.
- **FR-008**: Toolbar actions MUST deliver the same application event as today.
- **FR-009**: The window's title strip MUST be the window's own titlebar, not a view in the content tree.
- **FR-010**: The framework MUST expose the titlebar inset so content can clear the window controls in modern look mode.
- **FR-011**: No public API signature changes. `sidebar()`, `sidebar_item()`, `toolbar()` and `window()` keep their current builders; only their rendering changes.
- **FR-012**: The renderer MUST NOT position chrome elements by incrementing pixel offsets.
- **FR-013**: Font sizes and corner radii MUST come from `NSFont.systemFontSize`, `NSFont.smallSystemFontSize` and the design tokens, not from literals. **(Done — this feature's first slice.)**

### Key Entities

- **Sidebar item**: label, optional icon, selected state, handler.
- **Toolbar action**: identifier, label, optional icon, handler, separator flag, enabled flag.
- **Titlebar inset**: the vertical space the window controls occupy, reported to the application.

## Success Criteria *(mandatory)*

- **SC-001**: Zero occurrences of `x +=` or `y +=` positioning in the macOS renderer.
- **SC-002**: Zero literal font sizes in the macOS renderer; every font derives from the system or a caller-supplied size.
- **SC-003**: Sidebar selection animates; clicking through rows in the storybook shows a transition rather than an instant recolour.
- **SC-004**: Every demo application renders identically or better after the change, verified by screenshot before and after.
- **SC-005**: A window narrowed below its toolbar's natural width exposes the system overflow control.
- **SC-006**: The storybook and both application demos keep compiling and running with no application-side changes.

## Assumptions

- The layout authority rule still holds *inside* panes: AppKit owns the split and the chrome, taffy owns content layout within each pane. Using `NSSplitViewController` is not a violation of the layout invariant — the earlier `NSSplitView` bug came from using it *and* positioning its children from taffy, which this feature must not repeat.
- Applications that currently set `SidebarWidth` keep working; the prop becomes a hint bounded by the split view rather than an enforced pixel value.
- Windows and Linux backends are out of scope; they remain scaffolds.

## Out of Scope

- Grouped-section and card styling. AppKit exposes no public "card" control, so `NSBox` with semantic colours and a token radius is the accepted approach and stays.
- `NSPopover` for the `Popover` component, which is separately unimplemented.
- Any change to the flex layout engine.
