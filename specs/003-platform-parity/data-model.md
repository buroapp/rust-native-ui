# Phase 1 Data Model — Platform Parity

**Feature**: `003-platform-parity`
**Date**: 2026-11-03

This feature adds no persistent data and no new domain entities to the product. What it does add is a **capability model**: an explicit, checkable statement of which component kinds each backend implements, and the native control each one becomes.

---

## Entity: BackendCapability

The statement that a backend implements a given component kind.

| Field | Type | Notes |
|---|---|---|
| `kind` | `ComponentKind` | The platform-neutral component kind |
| `native_class` | `&'static str` | Win32 class name or GTK4 widget type name |
| `native_style` | `Option<&'static str>` | Win32 `BS_*` / `LVS_*` style, or GTK CSS class where load-bearing |
| `composed` | `bool` | True when the kind is assembled from multiple native controls rather than one |

**Invariants**

- The set of `BackendCapability.kind` values for a backend MUST be exactly the set of `ComponentKind` variants. Enforced by exhaustive `match` (compile time) and by a coverage test comparing `capabilities().len()` to the variant count.
- `composed == true` requires a corresponding row in the platform's composition table (research R1/R2) and MUST NOT imply custom painting.
- `native_class` MUST name a class provided by the OS or its standard toolkit. No entry may name a type defined by this project.

---

## Entity: ControlRecord

The per-node runtime record both backends keep in the render path.

| Field | Type | Notes |
|---|---|---|
| `handle` | `Option<Handle>` | `HWND` on Windows, `glib::Object`/`gtk4::Widget` on Linux |
| `kind` | `ComponentKind` | Retained so `update` and `delete` can dispatch without consulting the shadow tree |
| `is_composed` | `bool` | Composed kinds update N children from one logical node |

**Storage**: a dense `Vec<Option<ControlRecord>>` indexed by `NodeId.0`, grown geometrically. Cleared slot on delete. Justification in research R3.

**Invariants**

- The vector MUST be pre-sized before entering the render path; growth is permitted only when a node id exceeds capacity, which happens on tree growth, not on re-render.
- `handle == None` for a live node is a programming error and MUST surface as a diagnostic, never as a silently invisible control (FR-004).

---

## Entity: LayoutResult (existing, unchanged)

Produced by `rn-layout`, consumed identically by all backends. This feature does not modify it. It is listed because FR-008 makes it the sole source of geometry, and both new backends MUST apply it without recomputation.

| Field | Type |
|---|---|
| `x`, `y`, `width`, `height` | logical units, top-left origin |

---

## Windows Control Mapping

Normative. Every row MUST be implemented as stated. `⟳` marks a composed kind.

| # | ComponentKind | Win32 class | Style / notes |
|---|---|---|---|
| 1 | `Window` | top-level window | `WS_OVERLAPPEDWINDOW`, manifest-declared per-monitor DPI v2 |
| 2 | `Label` | `STATIC` | `SS_LEFT`, `SS_NOPREFIX` |
| 3 | `Button` | `BUTTON` | `BS_PUSHBUTTON` |
| 4 | `TextInput` | `EDIT` | `ES_AUTOHSCROLL`, `WS_TABSTOP` |
| 5 | `LayoutContainer` | child window | `WS_CHILD`, `WS_CLIPCHILDREN` |
| 6 | `Checkbox` | `BUTTON` | `BS_AUTOCHECKBOX` |
| 7 | `Slider` | `msctls_trackbar32` | `TBS_HORZ`, `TBS_AUTOTICKS` |
| 8 | `ProgressBar` | `msctls_progress32` | `PBS_SMOOTH` |
| 9 | `Separator` | `STATIC` | `SS_ETCHEDHORZ` / `SS_ETCHEDVERT` |
| 10 | `TextArea` | `EDIT` | `ES_MULTILINE \| ES_AUTOVSCROLL \| WS_VSCROLL` |
| 11 | `Dropdown` | `COMBOBOX` | `CBS_DROPDOWNLIST` |
| 12 | `Image` | `STATIC` | `SS_BITMAP` / `SS_ICON` + `STM_SETIMAGE` |
| 13 | `Stepper` | `msctls_updown32` + `EDIT` | `UDS_SETBUDDYINT` ⟳ |
| 14 | `ColorWell` | `BUTTON` + `ChooseColor` | Opens the common colour dialog ⟳ |
| 15 | `DatePicker` | `DATETIMEPICK_CLASS` | default date format |
| 16 | `RadioGroup` | `BUTTON` × n | `BS_AUTORADIOBUTTON`, first item `WS_GROUP` ⟳ |
| 17 | `ScrollView` | child window | `WS_VSCROLL`, `SCROLLINFO` |
| 18 | `FlatList` | `SysListView32` | `LVS_REPORT \| LVS_OWNERDATA` — native virtual list |
| 19 | `TouchableOpacity` | child window | Click via `WM_LBUTTONUP`; opacity via `WS_EX_LAYERED` |
| 20 | `SafeAreaView` | child window | Passthrough container |
| 21 | `ActivityIndicator` | `msctls_progress32` | `PBS_MARQUEE` — native indeterminate |
| 22 | `Modal` | `WS_POPUP` window | Owned by parent, `EnableWindow(parent, false)` |
| 23 | `Popover` | `WS_POPUP` window | `WS_EX_TOOLWINDOW`, dismiss on focus loss |
| 24 | `RichText` | `RICHEDIT50W` | `Msftedit.dll`, `EM_SETCHARFORMAT` |
| 25 | `Card` | `STATIC` | `SS_BLACKFRAME` + themed background |
| 26 | `Badge` | `STATIC` | `SS_CENTER`, themed fill |
| 27 | `Skeleton` | `STATIC` | Themed fill, no text |
| 28 | `SegmentedControl` | `BUTTON` × n | `BS_AUTORADIOBUTTON \| BS_PUSHLIKE \| BS_GROUP` ⟳ |
| 29 | `SearchBar` | `EDIT` | `EM_SETCUEBANNER` for placeholder |
| 30 | `Sidebar` | `SysListView32` | `LVS_REPORT`, `LVS_EX_FULLROWSELECT` |
| 31 | `SidebarItem` | `SysListView32` row | Inserted item with image index |
| 32 | `GroupedSection` | `BUTTON` `BS_GROUPBOX` | Native group frame |
| 33 | `SplitView` | 2 child windows + `STATIC` divider | Divider drag via mouse capture ⟳ |
| 34 | `Toolbar` | `msctls_toolbar` | `TBSTYLE_FLAT`, `BTNS_BUTTON` / `BTNS_SEP` |
| 35 | `Switch` | `BUTTON` | `BS_AUTOCHECKBOX \| BS_PUSHLIKE` ⟳ |
| 36 | `StatusBar` | `msctls_statusbar32` | `SB_SETPARTS` |
| 37 | `ContentArea` | child window | `WS_CLIPCHILDREN`, themed background |
| 38 | `ListTile` | `SysListView32` row | Icon + text columns ⟳ |
| 39 | `Scaffold` | child window | Vertical stack container |
| 40 | `TitleBar` | window chrome | `WS_CAPTION`; no child control |
| 41 | `IconButton` | `BUTTON` | `BS_ICON \| BS_PUSHLIKE` |
| 42 | `DisclosureButton` | `BUTTON` | `BS_AUTOCHECKBOX \| BS_PUSHLIKE` with chevron glyph ⟳ |
| 43 | `HelpButton` | `BUTTON` | `BS_PUSHBUTTON` with `?`; `WS_EX_CONTEXTHELP` on dialogs |
| 44 | `PopupButton` | `BUTTON` + `TrackPopupMenu` | Splits into a menu ⟳ |
| 45 | `BackButton` | `BUTTON` | `BS_ICON`, chevron glyph |
| 46 | `CapacityIndicator` | `msctls_progress32` | `PBS_SMOOTH`; discrete steps via `PBM_SETSTEP` |
| 47 | `RatingIndicator` | `BUTTON` × max | `BS_AUTORADIOBUTTON` + star glyph ⟳ |
| 48 | `RelevanceIndicator` | `msctls_progress32` | `PBS_SMOOTH` |
| 49 | `Tooltip` | `tooltips_class32` | `TTF_SUBCLASS` |
| 50 | `TimePicker` | `DATETIMEPICK_CLASS` | `DTS_TIMEFORMAT` |
| 51 | `GraphicalTimePicker` | `DATETIMEPICK_CLASS` + `msctls_updown32` | No clock face on Win32 ⟳ |
| 52 | `ToolbarIconButton` | `msctls_toolbar` button | `BTNS_BUTTON`, `TB_ADDBITMAP` |
| 53 | `ToolbarSpacer` | `msctls_toolbar` | `TBSTYLE_SEP` with width |
| 54 | `ToolbarDivider` | `msctls_toolbar` | `BTNS_SEP` |
| 55 | `ToolbarPopup` | `msctls_toolbar` button | `BTNS_DROPDOWN` + `TBN_DROPDOWN` |
| 56 | `AlertDialog` | `TaskDialogIndirect` | Native task dialog, command links |
| 57 | `Sheet` | `WS_POPUP` window | Modal, owned, positioned below caption |

---

## Linux Control Mapping

Normative. `⟳` marks a composed kind. libadwaita (`Adw*`) is used only where stock GTK4 has no adequate widget, and never for a widget GTK4 already provides.

| # | ComponentKind | GTK4 widget | Notes |
|---|---|---|---|
| 1 | `Window` | `GtkApplicationWindow` | `gtk4::Application`, Wayland and X11 backends |
| 2 | `Label` | `GtkLabel` | `set_xalign(0.0)`, ellipsize end |
| 3 | `Button` | `GtkButton` | |
| 4 | `TextInput` | `GtkEntry` | |
| 5 | `LayoutContainer` | `GtkBox` | orientation from `Axis` |
| 6 | `Checkbox` | `GtkCheckButton` | |
| 7 | `Slider` | `GtkScale` | `set_draw_value(true)` |
| 8 | `ProgressBar` | `GtkProgressBar` | |
| 9 | `Separator` | `GtkSeparator` | orientation from axis |
| 10 | `TextArea` | `GtkTextView` | inside `GtkScrolledWindow` |
| 11 | `Dropdown` | `GtkDropDown` | `GtkStringList` model |
| 12 | `Image` | `GtkImage` | icon-name or file |
| 13 | `Stepper` | `GtkSpinButton` | |
| 14 | `ColorWell` | `GtkColorButton` | Native colour button |
| 15 | `DatePicker` | `GtkMenuButton` + `GtkPopover` + `GtkCalendar` | GTK4 has no date entry widget ⟳ |
| 16 | `RadioGroup` | `GtkCheckButton` × n | grouped via `set_group` ⟳ |
| 17 | `ScrollView` | `GtkScrolledWindow` | `gtk4::PolicyType::Automatic` |
| 18 | `FlatList` | `GtkListView` + `GtkSingleSelection` | Native virtualisation via `GtkListItemFactory` |
| 19 | `TouchableOpacity` | `GtkBox` + `GtkGestureClick` | Opacity via `set_opacity` |
| 20 | `SafeAreaView` | `GtkBox` | Passthrough |
| 21 | `ActivityIndicator` | `GtkSpinner` | `start()` / `stop()` |
| 22 | `Modal` | `GtkWindow` | `set_modal(true)`, `set_transient_for` |
| 23 | `Popover` | `GtkPopover` | Anchored to parent widget |
| 24 | `RichText` | `GtkTextView` | `GtkTextTag` per styled span |
| 25 | `Card` | `GtkBox` | CSS class `card` |
| 26 | `Badge` | `GtkLabel` | CSS classes `badge` + `accent` |
| 27 | `Skeleton` | `GtkBox` | CSS class, themed fill |
| 28 | `SegmentedControl` | `GtkToggleButton` × n | `linked` style class ⟳ |
| 29 | `SearchBar` | `GtkSearchEntry` | Native search entry |
| 30 | `Sidebar` | `GtkListBox` | inside `GtkScrolledWindow`, `sidebar` style class |
| 31 | `SidebarItem` | `GtkListBoxRow` | Label + optional icon + badge |
| 32 | `GroupedSection` | `GtkBox` | CSS `card`; or `AdwPreferencesGroup` where available |
| 33 | `SplitView` | `GtkPaned` | Native paned container |
| 34 | `Toolbar` | `GtkHeaderBar` | Native header bar |
| 35 | `Switch` | `GtkSwitch` | Native switch |
| 36 | `StatusBar` | `GtkActionBar` | Native action bar |
| 37 | `ContentArea` | `GtkBox` | inside `GtkScrolledWindow` |
| 38 | `ListTile` | `GtkListBoxRow` | `GtkBox` + icon + labels + chevron ⟳ |
| 39 | `Scaffold` | `GtkBox` | Vertical stack |
| 40 | `TitleBar` | `GtkHeaderBar` | `set_title_widget`; CSD provides chrome |
| 41 | `IconButton` | `GtkButton` | `set_icon_name`, `flat` style class |
| 42 | `DisclosureButton` | `GtkToggleButton` | Expander chevron icon ⟳ |
| 43 | `HelpButton` | `GtkMenuButton` | `help-about-symbolic` icon |
| 44 | `PopupButton` | `GtkMenuButton` + `GtkPopoverMenu` | ⟳ |
| 45 | `BackButton` | `GtkButton` | `go-previous-symbolic` |
| 46 | `CapacityIndicator` | `GtkLevelBar` | `Continuous` → `Discrete` mode |
| 47 | `RatingIndicator` | `GtkToggleButton` × max | `linked` style class ⟳ |
| 48 | `RelevanceIndicator` | `GtkLevelBar` | Continuous mode |
| 49 | `Tooltip` | widget tooltip | `set_tooltip_text`, no widget |
| 50 | `TimePicker` | `GtkSpinButton` × 2 | Hours + minutes in a `GtkBox` ⟳ |
| 51 | `GraphicalTimePicker` | `GtkSpinButton` pair + `GtkCalendar` popover | GTK4 removed the clock face ⟳ |
| 52 | `ToolbarIconButton` | `GtkButton` in `GtkHeaderBar` | `pack_start` / `pack_end` |
| 53 | `ToolbarSpacer` | `GtkBox` | `hexpand(true)` |
| 54 | `ToolbarDivider` | `GtkSeparator` | Vertical in header bar |
| 55 | `ToolbarPopup` | `GtkMenuButton` in `GtkHeaderBar` | ⟳ |
| 56 | `AlertDialog` | `GtkAlertDialog` (4.10+) / `GtkMessageDialog` (4.8) | Version-gated |
| 57 | `Sheet` | `GtkWindow` | Modal, transient, no sheet concept in GTK4 ⟳ |

---

## Cross-platform state coverage

Properties that carry application state and MUST be applied on update, not only at creation (FR-009). Both backends implement the same table.

| Property | Windows update path | Linux update path |
|---|---|---|
| `Text` | `WM_SETTEXT` / `SetWindowTextW` | `label.set_text`, `entry.set_text` |
| `Value` | `TBM_SETPOS`, `PBM_SETPOS`, `UDM_SETPOS` | `scale.set_value`, `spin.set_value` |
| `Checked` | `BM_SETCHECK` | `check.set_active`, `switch.set_active` |
| `SwitchValue` | `BM_SETCHECK` | `switch.set_active` |
| `SelectedIndex` | `CB_SETCURSEL`, `LVM_SETITEMSTATE`, `TCM_SETCURSEL` | `dropdown.set_selected`, `listbox.select_row` |
| `Selected` | `LVM_SETITEMSTATE` | `row.set_selected` |
| `Enabled` | `EnableWindow` | `set_sensitive` |
| `Visible` | `ShowWindow` | `set_visible` |
| `Options` | `CB_RESETCONTENT` + `CB_ADDSTRING` | rebuild `GtkStringList` |
| `DateValue` | `DTM_SETSYSTEMTIME` | `calendar.select_day` |
| `ImageName` | `STM_SETIMAGE` | `image.set_icon_name` |
| `Spans` (RichText) | `EM_SETCHARFORMAT` per run | `GtkTextTag` per run |
| `BadgeLabel` | `WM_SETTEXT` on badge static | `label.set_text` on badge |
| `SectionTitle` | `WM_SETTEXT` on group box | group header label |
