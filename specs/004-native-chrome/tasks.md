# Tasks: Native Chrome — Sidebar, Toolbar, Titlebar

**Feature**: `004-native-chrome` | **Input**: spec.md

Derived from the spec's functional requirements. Ordered by dependency, then by
user-story priority.

## Phase 1 — Setup

- [X] T001 Point `.specify/feature.json` at `specs/004-native-chrome`
- [X] T002 Confirm the `NSToolbar` / `NSToolbarItem` / `NSToolbarDelegate` bindings and
      objc2-app-kit features are available (FR-006)

## Phase 2 — Toolbar (User Story 2, P1)

- [X] T003 Declare a `ToolbarDelegate` class holding the declared actions and the
      toolbar items it vends (FR-006)
- [X] T004 Implement `toolbarDefaultItemIdentifiers` and
      `toolbarAllowedItemIdentifiers` from the action list, mapping spacer actions to
      the system flexible-space identifier and separators to the system separator
      identifier (FR-006, FR-007)
- [X] T005 Implement `toolbar:itemForItemIdentifier:willBeInsertedIntoToolbar:` to
      build an `NSToolbarItem` with label, SF Symbol image and the action's handler
      wired to the existing event path (FR-006, FR-008)
- [X] T006 Attach the toolbar to the window that owns the component, with the unified
      toolbar style so it merges with the titlebar (FR-009)
- [X] T007 Stop the `Toolbar` component rendering a strip in the content tree: its
      layout height becomes zero so the window chrome owns the space (FR-009,
      FR-012)
- [X] T008 Verify every demo that declares a toolbar still launches and that its
      actions fire (SC-002, SC-006)

## Phase 3 — Titlebar (User Story 3, P2)

- [ ] T009 Remove the hand-drawn title strip from the content tree and rely on the
      window's own titlebar (FR-009)
- [ ] T010 Expose the titlebar inset so content can clear the window controls in
      modern look mode (FR-010)

## Phase 4 — Sidebar (User Story 1, P1)

- [X] T011 Host sidebar rows in an `NSOutlineView` with source-list style inside an
      `NSSplitViewController` sidebar item (FR-001, FR-003, FR-004).
      The sidebar pane is an `NSScrollView` whose document is an `NSOutlineView`
      (`NSTableViewStyle::SourceList`). A `SidebarModel` NSObject is its data source
      and delegate, built at promote time from the item/section children. Rows,
      cells, selection, keyboard navigation and group disclosure are AppKit's.
- [X] T012 Feed row label, icon and selected state from the existing props so
      applications keep working unchanged (FR-002, FR-005). `build_sidebar_data`
      reads Text/IconName/OnSelect/Selected; selection fires `OnSelect` through the
      existing event path (verified: AX row select → `outlineViewSelectionDidChange`
      → dispatch). Initial `Selected` prop becomes the outline's selected row.
- [X] T013 Delete the `NSBox` pill and the manual selection repaint it replaced
      (FR-003, FR-012). `SidebarItem`/`SidebarSection` no longer build views — they
      are 0x0 placeholders read as data; the `update_node` pill repaint is gone; the
      `SidebarItemWidget` three-field variant collapsed to one.

## Phase 5 — Polish

- [x] T014 Fonts derive from `NSFont.systemFontSize` (FR-013) — done in `bff82ab`
- [x] T015 Radii derive from `RadiusToken` (FR-013) — done in `bff82ab`
- [ ] T016 Confirm zero `x +=`/`y +=` positioning and zero literal font sizes remain
      (SC-001, SC-002)

## Dependencies

- T003 → T004 → T005 → T006 → T007 → T008 (toolbar is one chain)
- T011 → T012 → T013 (sidebar is one chain, independent of the toolbar)
- T009/T010 depend on T006 (the window's toolbar must exist before the title strip
  can be removed)

## Notes

Phase 3 and 4 are not started by this run. Phases 1 and 2 are the P1 toolbar work.

Known consequence of T007: a demo that declares `toolbar(actions)` no longer renders an
inline strip, because the window owns the toolbar now. `mail-demo` verified. The
storybook catalog's Toolbar demo therefore installs the catalog window's toolbar when
it is opened, which is correct behaviour but looks empty in the page body.
