# Plan: Sidebar as a source-list split item

**Feature**: `004-native-chrome` | Covers T011–T013

## Why this is one job, not three

The toolbar covers the sidebar's top corner because `NSToolbar` paints its own
background across the full window width. No frame manipulation beats that. A sidebar
that reaches the top of the window is a *split-view sidebar item* — AppKit draws that
item above the toolbar band. So "sidebar to the top" and "sidebar rows are a real
source list" are the same change: both require the sidebar to stop being a view we lay
out by hand and become a split-view item hosting an `NSOutlineView`.

## Preconditions

Enable these `objc2-app-kit` features in `rn-macos/Cargo.toml` — none are currently on:

- `NSSplitViewController`, `NSSplitViewItem`
- `NSOutlineView`, `NSTableView`, `NSTableColumn`, `NSTableCellView`, `NSTableRowView`
- `NSImageView` (cell images), `NSVisualEffectView` (already used, verify)

## Shape

1. **Window level.** When a window's root subtree is exactly
   `hstack[Sidebar, <separator?>, …]`, build an `NSSplitViewController` with two items:
   a sidebar item (`NSSplitViewItem::sidebarWithViewController`) and a content item.
   Set `splitView.autosaveName` and a `holdingPriority` on the content item so resizing
   the window grows content, not the sidebar.

2. **Sidebar item.** Host the existing vibrancy surface plus an `NSScrollView` whose
   document is an `NSOutlineView`:
   - `setHeaderView(None)`
   - `NSTableViewStyle::SourceList`
   - one `NSTableColumn` with `resizingMask = AutoresizingMask`
   - `setOutlineTableColumn(column)`
   - `setIndentationPerLevel` at the system source-list value, not a literal

3. **Rows from the tree.** `Sidebar` becomes `forces_no_children` in `build_spec`, the
   same escape hatch `TabView` already uses: `NSOutlineView` owns row geometry, so
   taffy must not also compute rects for those children.
   A `SidebarSource` class implements the data source and delegate:
   - `sidebar_section` → root item (group row, no disclosure triangle, small caps)
   - `sidebar_item` within a section → child row
   - `sidebar_item` before any section → child of an implicit root group
   - `viewForTableColumn:item:` builds an `NSTableCellView` with an `NSImageView`
     (SF Symbol from `PropKey::IconName`) and an `NSTextField` label
   - `outlineViewSelectionDidChange:` fires the item node's `OnSelect` handler through
     the existing event path

4. **Delete the imitation.** Remove the `NSBox` pill construction for `SidebarItem`,
   the `PropKey::Selected` repaint arm in `update_node`, and the manual
   `scrollRectToVisible` reveal in `ensure_selection_visible` for sidebar rows —
   `NSOutlineView` draws selection and scrolls it into view itself.

## Migration notes

- Four demos use `sidebar_item` children: `storybook`, `settings-demo`, `mail-demo`,
  `finder-demo`. Their public API does not change; only rendering does.
- `sidebar_section` currently renders a 18pt label band. It becomes a group row.
- `SidebarItem`'s `Selected` prop becomes the outline's selection, set once on build
  and then owned by AppKit.

## Verification

- Sidebar surface reaches y=0 under the traffic lights; toolbar band does not cover it.
- Clicking a row fires `OnSelect` and moves the highlight (AX-check a row's selected state).
- `storybook --select Sidebar` and `--select SidebarSection` still render.
- Resizing the window grows the content pane, not the sidebar.
- Clippy clean, workspace tests pass, catalog drift test still passes.

## Status

Not started. `main` is clean at the last commit; this plan is written so the work can
begin without re-deriving the approach.
