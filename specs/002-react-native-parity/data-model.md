# Data Model: React Native Component Parity + Shadcn Design System

**Feature**: `002-react-native-parity` | **Phase**: 1

---

## Design System Tokens

### ColorToken

```
ColorToken {
    Primary      // main brand / accent color
    Secondary    // secondary action color
    Destructive  // danger / error (red family)
    Muted        // subdued text and backgrounds
    Background   // window / panel background
    Foreground   // primary text color
    Card         // card surface color
    Border       // subtle divider color
    Success      // positive state (green family)
    Warning      // cautionary state (amber family)
}
```

Platform mappings:
- macOS: `NSColor.systemAccent`, `NSColor.systemRed`, `NSColor.secondaryLabelColor`, `NSColor.windowBackgroundColor`, etc.
- Windows: `DWMWA_CAPTION_COLOR`, Win32 system colors
- Linux: GTK CSS `@theme_bg_color`, `@theme_fg_color`, `@error_color`, etc.

### SpacingToken

```
SpacingToken {
    None  = 0
    Xs    = 4px
    Sm    = 8px
    Md    = 12px
    Lg    = 16px
    Xl    = 20px
    Xxl   = 24px
    Xxxl  = 32px
}
```

### RadiusToken

```
RadiusToken {
    None  = 0px
    Sm    = 2px
    Md    = 4px
    Lg    = 6px
    Xl    = 8px
    Full  = 9999px (circle)
}
```

### FontToken

```
FontToken {
    Body         // platform default body font, 14pt
    BodySm       // 12pt
    HeadingSm    // 16pt semibold
    HeadingMd    // 20pt semibold
    HeadingLg    // 24pt bold
    Mono         // monospace, 13pt
}
```

Platform mappings:
- macOS: `.systemFont(ofSize:)`, `.monospacedSystemFont(ofSize:)`
- Windows: `LOGFONT` + Segoe UI
- Linux: GTK `pango_font_description_from_string`

### FocusRingToken

```
FocusRingToken {
    width:  f32,        // 2px default
    color:  ColorToken, // Primary default
    style:  FocusRingStyle { Solid, Dashed }
}
```

### Style (builder)

```
Style {
    color:       Option<ColorToken>
    bg_color:    Option<ColorToken>
    spacing:     Option<SpacingToken>      // padding
    gap:         Option<SpacingToken>      // inter-child gap
    radius:      Option<RadiusToken>
    font:        Option<FontToken>
    focus_ring:  Option<FocusRingToken>
    bold:        bool
    italic:      bool
    opacity:     Option<f32>               // 0.0–1.0, for TouchableOpacity
    shadow:      Option<ShadowStyle>
}

ShadowStyle {
    offset_x:  f32,
    offset_y:  f32,
    blur:      f32,
    color:     ColorToken
}
```

---

## Core UI Components (new ComponentKind variants)

### ScrollView

```
ScrollViewProps {
    direction:      ScrollDirection { Vertical, Horizontal, Both }
    scrollable:     bool              // default true
    content_offset: Option<Point>     // programmatic scroll
    on_scroll:      Option<HandlerId>
}
```

Native: `NSScrollView` (macOS), `WS_VSCROLL`/`WS_HSCROLL` (Win32), `GtkScrolledWindow` (Linux)

### FlatList

```
FlatListProps {
    data:                Vec<Value>          // serialisable data items
    item_height:         ItemHeight { Fixed(f32) | Dynamic }
    key_extractor:       HandlerId           // handler returns String key for item
    render_item:         HandlerId           // handler returns Component for item
    on_end_reached:      Option<HandlerId>
    end_reach_threshold: f32                 // px from end, default 100
    buffer_rows:         u32                 // rows to pre-render beyond viewport, default 3
}

ItemHeight {
    Fixed(f32)   // constant row height, O(1) scroll position calculation
    Dynamic      // measured per row; height cache maintained by renderer
}
```

Native: virtual overlay over `NSScrollView` + `NSView` pool (macOS), `WS_VSCROLL` + HWND pool (Win32), `GtkListView` with `GtkListItemFactory` (Linux)

### TouchableOpacity

```
TouchableOpacityProps {
    on_press:       HandlerId
    opacity:        f32         // 0.0–1.0, default 0.4 (pressed state)
    stop_bubbling:  bool        // default false
}
```

Native: wrapper view with alpha animation on press (macOS `NSView.alphaValue`, Win32 `UpdateLayeredWindow`, GTK `gtk_widget_set_opacity`)

### SafeAreaView

```
SafeAreaViewProps {
    // read-only from platform at render time
    insets: Insets { top: f32, right: f32, bottom: f32, left: f32 }
}
```

Native: inset calculated from `NSWindow.contentLayoutGuide` (macOS), `GetSystemMetrics(SM_CYCAPTION)` (Win32), `gtk_widget_get_allocation` + panel height query (Linux)

### ActivityIndicator

```
ActivityIndicatorProps {
    visible: bool
    color:   Option<ColorToken>
    size:    IndicatorSize { Sm, Md, Lg }
}
```

Native: `NSProgressIndicator` (indeterminate, macOS), Win32 `PBS_MARQUEE` ProgressBar, `GtkSpinner` (Linux)

### Modal

```
ModalProps {
    visible:               bool
    on_dismiss:            Option<HandlerId>
    close_on_backdrop:     bool
    position:              ModalPosition { Center, Top, Bottom, Popover }
}

ModalPosition { Center, Top, Bottom, Popover }
```

Native: `NSPanel` (sheet or floating, macOS), Win32 modal dialog, `GtkDialog` (Linux)

### Popover

```
PopoverProps {
    anchor_id:  NodeId          // target node to anchor to
    offset:     Point
    on_dismiss: Option<HandlerId>
}
```

Native: `NSPopover` (macOS), Win32 popup window, `GtkPopover` (Linux)

### Switch

```
SwitchProps {
    value:     bool
    on_change: HandlerId
    disabled:  bool
}
```

Native: `NSSwitch` (macOS 10.15+), Win32 toggle button, GTK `GtkSwitch` (Linux)

### StatusBar

```
StatusBarProps {
    visible:          bool
    background_color: Option<ColorToken>
}
```

Native: macOS `NSWindow.titlebarAppearsTransparent`, Win32 DWM caption color; no-op on Linux

### Text (extended from Label)

```
TextProps {
    spans:    Vec<StyledSpan>
    wrap:     bool              // default true
    on_press: Option<HandlerId>
    align:    TextAlign { Left, Center, Right, Justified }
    max_lines: Option<u32>
}

StyledSpan {
    text:        String
    bold:        bool
    italic:      bool
    underline:   bool
    color:       Option<ColorToken>
    font:        Option<FontToken>
    font_size:   Option<f32>
}
```

Native: `NSAttributedString` + `NSTextView` (macOS), Win32 `DrawText` with `LOGFONT`, Pango spans (Linux)

---

## Design System Components

### Card

```
CardProps {
    padding:       SpacingToken  // default Lg (16px)
    shadow:        Option<ShadowStyle>
    border_radius: RadiusToken   // default Lg (6px)
    bg_color:      ColorToken    // default Card
}
```

Native: `NSBox` with shadow + corner radius (macOS), Win32 layered child window, GTK `GtkFrame` (Linux)

### Badge

```
BadgeProps {
    label:         String
    color_variant: BadgeVariant { Default, Secondary, Destructive, Success, Warning }
    size:          BadgeSize { Sm, Md, Lg }
    on_press:      Option<HandlerId>
}
```

Native: `NSTextField` with `backgroundColor` + `layer.cornerRadius` (macOS), Win32 Static + custom draw, GTK `GtkLabel` + CSS (Linux)

### Skeleton

```
SkeletonProps {
    width:         f32
    height:        f32
    radius:        RadiusToken   // default Md (4px)
    color_variant: SkeletonVariant { Muted, Card }
}
```

Native: `NSView` with `CAGradientLayer` animation (macOS), Win32 `AnimateWindow`, GTK CSS `@keyframes` pulse (Linux)

---

## Structural Layout Components

### SegmentedControl

```
SegmentedControlProps {
    options:        Vec<String>
    selected_index: i32
    on_change:      HandlerId
    disabled:       bool
}
```

Native: `NSSegmentedControl` (macOS), Win32 toolbar buttons, GTK `GtkStackSwitcher` (Linux)

### SearchBar

```
SearchBarProps {
    placeholder:  String
    value:        String
    on_change:    HandlerId
    on_clear:     Option<HandlerId>
    on_submit:    Option<HandlerId>
}
```

Native: `NSSearchField` (macOS), Win32 EditBox with search icon, GTK `GtkSearchEntry` (Linux)

### Sidebar

```
SidebarProps {
    width:     f32       // default 220px
    min_width: f32       // default 160px
    max_width: f32       // default 320px
    collapsed: bool
}
```

Native: `NSSplitView` leading pane with `NSSidebarViewController` style (macOS), Win32 TreeView/ListBox panel, GTK `GtkListBox` in a pane (Linux)

### SidebarItem

```
SidebarItemProps {
    label:     String
    icon_name: Option<String>    // SF Symbol name (macOS) / Adwaita icon (Linux)
    selected:  bool
    on_select: HandlerId
    badge:     Option<String>    // badge label, e.g. "99+"
}
```

Native: `NSTableCellView` row within sidebar table (macOS), Win32 `TVITEM`, GTK `GtkListBoxRow` (Linux)

### GroupedSection

```
GroupedSectionProps {
    title:   Option<String>   // optional section header label
    padding: SpacingToken     // default Md (12px)
}
```

Native: `NSBox` with `.groupBox` style + rounded corners (macOS), Win32 GroupBox, GTK `GtkFrame` + `GtkListBox` (Linux)

### SplitView

```
SplitViewProps {
    orientation:   SplitOrientation { Horizontal, Vertical }
    divider_width: f32       // default 1px
    min_pane_size: Vec<f32>  // min size per pane
    pane_sizes:    Vec<f32>  // current size per pane (programmatic control)
    on_resize:     Option<HandlerId>
}
```

Native: `NSSplitView` (macOS), Win32 CSplitter, `GtkPaned` (Linux)

### Toolbar

```
ToolbarProps {
    actions: Vec<ToolbarAction>
}

ToolbarAction {
    id:        String
    label:     Option<String>
    icon_name: Option<String>
    on_press:  HandlerId
    separator: bool    // if true, renders a divider at this position
    enabled:   bool
}
```

Native: `NSToolbar` with `NSToolbarItem` (macOS), Win32 `TBBUTTON` toolbar, GTK `GtkToolbar` / `GtkHeaderBar` (Linux)

---

## Event Model (new variants)

```
EventPayload (existing) + new variants:
    Scroll(ScrollEvent)     // from ScrollView.on_scroll
    Search(String)          // from SearchBar.on_change
    Segment(i32)            // from SegmentedControl.on_change
    SplitResize(Vec<f32>)   // from SplitView.on_resize
    ToolbarAction(String)   // from Toolbar action id

ScrollEvent {
    offset_x: f32
    offset_y: f32
    velocity_x: f32
    velocity_y: f32
}
```

---

## State Transitions

### FlatList Virtualization State Machine

```
FlatListState {
    visible_range:  Range<usize>   // indices of visible items
    buffer_range:   Range<usize>   // visible + buffer
    live_pool:      Vec<NodeId>    // recycled component nodes
    height_cache:   HashMap<usize, f32>  // measured heights for Dynamic mode
    total_height:   f32            // sum of all item heights
    scroll_offset:  f32            // current scroll position
}

Transitions:
- OnScroll → update visible_range, recycle out-of-range nodes, mount in-range nodes
- OnDataAppend → update total_height, recompute visible_range
- OnDataUpdate(index) → if in visible_range: update live node; else: mark dirty, apply on next mount
- OnItemHeightChange(index) → update height_cache, recompute all downstream offsets
```

### Modal Visibility State Machine

```
ModalState { Hidden | Presenting | Visible | Dismissing }

Transitions:
- Hidden + visible=true → Presenting → Visible
- Visible + visible=false → Dismissing → Hidden
- Visible + backdrop_click + close_on_backdrop=true → Dismissing → Hidden
```
