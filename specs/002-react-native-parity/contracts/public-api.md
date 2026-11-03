# Public API Contract: React Native Component Parity + Shadcn Design System

**Feature**: `002-react-native-parity` | **Phase**: 1
**Extends**: `specs/001-core-framework/contracts/public-api.md`

All APIs belong to the `rn` crate (public, platform-agnostic). No platform types may appear here.

---

## Design System Tokens

```rust
// rn::style module

pub enum ColorToken {
    Primary, Secondary, Destructive,
    Muted, Background, Foreground,
    Card, Border, Success, Warning,
}

pub enum SpacingToken {
    None, Xs, Sm, Md, Lg, Xl, Xxl, Xxxl,
}

pub enum RadiusToken {
    None, Sm, Md, Lg, Xl, Full,
}

pub enum FontToken {
    Body, BodySm, HeadingSm, HeadingMd, HeadingLg, Mono,
}

pub struct FocusRingToken {
    pub width: f32,
    pub color: ColorToken,
    pub style: FocusRingStyle,
}

pub enum FocusRingStyle { Solid, Dashed }

pub struct ShadowStyle {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub color: ColorToken,
}

pub struct Style {
    // builder pattern — all fields Option
}

impl Style {
    pub fn new() -> Self;
    pub fn primary() -> Self;    // pre-baked presets
    pub fn secondary() -> Self;
    pub fn destructive() -> Self;
    pub fn ghost() -> Self;
    pub fn outline() -> Self;

    pub fn color(self, c: ColorToken) -> Self;
    pub fn bg_color(self, c: ColorToken) -> Self;
    pub fn spacing(self, s: SpacingToken) -> Self;
    pub fn gap(self, s: SpacingToken) -> Self;
    pub fn rounded(self, r: RadiusToken) -> Self;
    pub fn font(self, f: FontToken) -> Self;
    pub fn focus_ring(self, t: FocusRingToken) -> Self;
    pub fn bold(self) -> Self;
    pub fn italic(self) -> Self;
    pub fn opacity(self, v: f32) -> Self;
    pub fn shadow(self, s: ShadowStyle) -> Self;
}
```

---

## ComponentBuilder extensions

All existing `ComponentBuilder` methods are preserved. New methods added:

```rust
impl ComponentBuilder {
    pub fn style(self, s: Style) -> Self;
}
```

---

## New Builder Functions

### ScrollView

```rust
pub fn scroll_view() -> ComponentBuilder;

// builder methods:
.direction(ScrollDirection)   // Vertical (default), Horizontal, Both
.scrollable(bool)
.content_offset(Point)
.on_scroll(HandlerId)
.child(Component)
```

### FlatList

```rust
pub fn flat_list<T: serde::Serialize>(
    data: Vec<T>,
    render_item: HandlerId,
) -> ComponentBuilder;

// builder methods:
.item_height(ItemHeight)          // Fixed(f32) or Dynamic
.key_extractor(HandlerId)
.on_end_reached(HandlerId)
.end_reach_threshold(f32)
.buffer_rows(u32)
```

### TouchableOpacity

```rust
pub fn touchable_opacity(on_press: HandlerId) -> ComponentBuilder;

// builder methods:
.opacity(f32)          // pressed opacity, default 0.4
.stop_bubbling(bool)
.child(Component)
```

### SafeAreaView

```rust
pub fn safe_area_view() -> ComponentBuilder;

// builder methods:
.child(Component)
```

### ActivityIndicator

```rust
pub fn activity_indicator() -> ComponentBuilder;

// builder methods:
.visible(bool)
.color(ColorToken)
.size(IndicatorSize)   // Sm, Md, Lg
```

### Modal

```rust
pub fn modal(visible: bool) -> ComponentBuilder;

// builder methods:
.on_dismiss(HandlerId)
.close_on_backdrop(bool)
.position(ModalPosition)    // Center, Top, Bottom, Popover
.child(Component)
```

### Popover

```rust
pub fn popover(anchor_id: NodeId) -> ComponentBuilder;

// builder methods:
.offset(Point)
.on_dismiss(HandlerId)
.child(Component)
```

### Text (extended)

```rust
pub fn text(spans: Vec<StyledSpan>) -> ComponentBuilder;
pub fn text_plain(s: impl Into<String>) -> ComponentBuilder;  // convenience

// builder methods:
.wrap(bool)
.align(TextAlign)          // Left, Center, Right, Justified
.max_lines(u32)
.on_press(HandlerId)
```

```rust
pub struct StyledSpan {
    pub text:       String,
    pub bold:       bool,
    pub italic:     bool,
    pub underline:  bool,
    pub color:      Option<ColorToken>,
    pub font:       Option<FontToken>,
    pub font_size:  Option<f32>,
}

impl StyledSpan {
    pub fn new(text: impl Into<String>) -> Self;
    pub fn bold(self) -> Self;
    pub fn italic(self) -> Self;
    pub fn underline(self) -> Self;
    pub fn color(self, c: ColorToken) -> Self;
    pub fn font(self, f: FontToken) -> Self;
    pub fn size(self, px: f32) -> Self;
}
```

### Design System Components

```rust
pub fn card() -> ComponentBuilder;
// builder methods:
.padding(SpacingToken)
.shadow(ShadowStyle)
.border_radius(RadiusToken)
.bg_color(ColorToken)
.child(Component)

pub fn badge(label: impl Into<String>) -> ComponentBuilder;
// builder methods:
.variant(BadgeVariant)    // Default, Secondary, Destructive, Success, Warning
.size(BadgeSize)          // Sm, Md, Lg
.on_press(HandlerId)

pub fn skeleton(width: f32, height: f32) -> ComponentBuilder;
// builder methods:
.radius(RadiusToken)
.variant(SkeletonVariant)   // Muted, Card
```

### Structural Layout Components

```rust
pub fn segmented_control(
    options: Vec<String>,
    selected_index: i32,
    on_change: HandlerId,
) -> ComponentBuilder;
// builder methods:
.disabled(bool)

pub fn search_bar(placeholder: impl Into<String>, on_change: HandlerId) -> ComponentBuilder;
// builder methods:
.value(impl Into<String>)
.on_clear(HandlerId)
.on_submit(HandlerId)

pub fn sidebar() -> ComponentBuilder;
// builder methods:
.width(f32)
.min_width(f32)
.max_width(f32)
.collapsed(bool)
.child(Component)   // expects SidebarItem children

pub fn sidebar_item(label: impl Into<String>, on_select: HandlerId) -> ComponentBuilder;
// builder methods:
.icon_name(impl Into<String>)
.selected(bool)
.badge(impl Into<String>)

pub fn grouped_section() -> ComponentBuilder;
// builder methods:
.title(impl Into<String>)
.padding(SpacingToken)
.child(Component)

pub fn split_view() -> ComponentBuilder;
// builder methods:
.orientation(SplitOrientation)    // Horizontal (default), Vertical
.divider_width(f32)
.min_pane_sizes(Vec<f32>)
.pane_sizes(Vec<f32>)
.on_resize(HandlerId)
.child(Component)    // 2–3 children = panes

pub fn toolbar(actions: Vec<ToolbarAction>) -> ComponentBuilder;
```

```rust
pub struct ToolbarAction {
    pub id:        String,
    pub label:     Option<String>,
    pub icon_name: Option<String>,
    pub on_press:  HandlerId,
    pub separator: bool,
    pub enabled:   bool,
}
```

---

## Event Payload Extensions

```rust
// Extends existing rn::event::EventPayload

pub enum EventPayload {
    // existing variants ...
    Scroll(ScrollEvent),
    Search(String),
    Segment(i32),
    SplitResize(Vec<f32>),
    ToolbarAction(String),
}

pub struct ScrollEvent {
    pub offset_x:   f32,
    pub offset_y:   f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
}
```

---

## Breaking Changes

None. All additions are new `ComponentKind` variants and new `EventPayload` variants. Existing consumers see no changes to existing APIs.

---

## Version Impact

This adds new `ComponentKind` variants — a non-breaking addition per semver conventions for the `rn` crate. The public API version bumps to `0.2.0` (minor bump, new features, no breaks).
