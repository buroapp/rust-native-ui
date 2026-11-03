//! Core types shared between `rn` and its platform backend crates.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// NodeId
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

impl NodeId {
    pub const UNASSIGNED: NodeId = NodeId(u64::MAX);

    #[doc(hidden)]
    pub fn for_test(value: u64) -> Self {
        NodeId(value)
    }
}

// ---------------------------------------------------------------------------
// Component kinds + props
// ---------------------------------------------------------------------------

/// Widget kinds understood by the framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentKind {
    Window,
    Label,
    Button,
    TextInput,
    LayoutContainer(Axis),
    // Phase 1 (v0.1)
    Checkbox,
    Slider,
    ProgressBar,
    Separator,
    TextArea,
    Dropdown,
    Image,
    Stepper,
    ColorWell,
    DatePicker,
    RadioGroup,
    // Phase 2 (v0.2) — RN parity + design system
    ScrollView,
    FlatList,
    TouchableOpacity,
    SafeAreaView,
    ActivityIndicator,
    Modal,
    Popover,
    RichText,
    Card,
    Badge,
    Skeleton,
    SegmentedControl,
    SearchBar,
    Sidebar,
    SidebarItem,
    GroupedSection,
    SplitView,
    Toolbar,
    Switch,
    StatusBar,
    // Batch 1 — macOS layout primitives (macos_ui parity)
    ContentArea,
    ListTile,
    Scaffold,
    TitleBar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropKey {
    // Layout / visibility
    Text,
    Enabled,
    Visible,
    Width,
    Height,
    MinWidth,
    MinHeight,
    Padding,
    Gap,
    FlexGrow,   // f32 — how much this node grows within its parent flex container
    FlexShrink, // f32 — how much this node shrinks
    Justify,    // f32 — main-axis alignment: 0=start 1=end 2=center 3=space-between
    Title,
    Placeholder,
    Subtitle,   // secondary label text (ListTile)
    ShowChevron, // bool — trailing disclosure chevron (ListTile)
    // Handlers
    OnPress,
    OnChange,
    OnClose,
    // Numeric range & value
    Value,
    Min,
    Max,
    Step,
    // Boolean state
    Checked,
    // Selection
    SelectedIndex,
    Options,
    // Media / appearance
    ImageName,
    Color,
    DateValue,
    // Phase 2 — scroll
    Direction,
    Scrollable,
    ContentOffset,
    OnScroll,
    // Phase 2 — flat list
    ItemHeightFixed,
    ItemHeightDynamic,
    BufferRows,
    EndReachThreshold,
    OnEndReached,
    // Phase 2 — touchable
    PressOpacity,
    StopBubbling,
    // Phase 2 — activity indicator
    IndicatorSize,
    // Phase 2 — modal / popover
    ModalVisible,
    ModalPosition,
    CloseOnBackdrop,
    OnDismiss,
    AnchorId,
    // Phase 2 — rich text
    TextSpans,
    TextWrap,
    TextAlign,
    MaxLines,
    // Phase 2 — design tokens
    StylePadding,
    StyleGap,
    StyleRadius,
    StyleBgColor,
    StyleFgColor,
    StyleFont,
    StyleShadow,
    StyleOpacity,
    // Phase 2 — badge / skeleton
    Variant,
    BadgeLabel,
    // Phase 2 — sidebar
    SidebarWidth,
    SidebarMinWidth,
    SidebarMaxWidth,
    Collapsed,
    IconName,
    Selected,
    OnSelect,
    SidebarBadge,
    SectionTitle,
    // Phase 2 — split view
    SplitOrientation,
    DividerWidth,
    MinPaneSizes,
    PaneSizes,
    OnResize,
    // Phase 2 — toolbar
    ToolbarActions,
    // Phase 2 — status bar
    BackgroundColor,
    // Phase 2 — search bar
    SearchValue,
    OnClear,
    OnSubmit,
    // Phase 2 — switch
    SwitchValue,
    // Phase 2 — FlatList callbacks
    RenderItem,
    KeyExtractor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandlerId(pub u64);

impl HandlerId {
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        HandlerId(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for HandlerId {
    fn default() -> Self {
        Self::new()
    }
}

/// RGBA color with each channel in 0–255.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorValue {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ColorValue {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// A typed value stored in a component's prop map.
#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    String(String),
    Bool(bool),
    F32(f32),
    HandlerRef(HandlerId),
    StringList(Vec<String>),
    Color(ColorValue),
    // Phase 2
    NodeRef(NodeId),
    Spans(Vec<StyledSpan>),
    ToolbarActions(Vec<ToolbarAction>),
    F32List(Vec<f32>),
    ColorToken(ColorToken),
    SpacingToken(SpacingToken),
    RadiusToken(RadiusToken),
    FontToken(FontToken),
}

// ---------------------------------------------------------------------------
// Phase 2 auxiliary types
// ---------------------------------------------------------------------------

/// Semantic color tokens for the design system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorToken {
    Primary,
    Secondary,
    Destructive,
    Muted,
    Background,
    Foreground,
    Card,
    Border,
    Success,
    Warning,
}

/// Spacing scale values (px).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpacingToken {
    None,
    Xs,   // 4
    Sm,   // 8
    Md,   // 12
    Lg,   // 16
    Xl,   // 20
    Xxl,  // 24
    Xxxl, // 32
}

impl SpacingToken {
    pub fn px(self) -> f32 {
        match self {
            SpacingToken::None => 0.0,
            SpacingToken::Xs   => 4.0,
            SpacingToken::Sm   => 8.0,
            SpacingToken::Md   => 12.0,
            SpacingToken::Lg   => 16.0,
            SpacingToken::Xl   => 20.0,
            SpacingToken::Xxl  => 24.0,
            SpacingToken::Xxxl => 32.0,
        }
    }
}

/// Border radius tokens (px).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RadiusToken {
    None,
    Sm,   // 2
    Md,   // 4
    Lg,   // 6
    Xl,   // 8
    Full, // 9999
}

impl RadiusToken {
    pub fn px(self) -> f32 {
        match self {
            RadiusToken::None => 0.0,
            RadiusToken::Sm   => 2.0,
            RadiusToken::Md   => 4.0,
            RadiusToken::Lg   => 6.0,
            RadiusToken::Xl   => 8.0,
            RadiusToken::Full => 9999.0,
        }
    }
}

/// Typography tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontToken {
    Body,
    BodySm,
    HeadingSm,
    HeadingMd,
    HeadingLg,
    Mono,
}

/// A styled text span for use in `RichText` components.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledSpan {
    pub text:      String,
    pub bold:      bool,
    pub italic:    bool,
    pub underline: bool,
    pub color:     Option<ColorToken>,
    pub font:      Option<FontToken>,
    pub font_size: Option<f32>,
}

impl StyledSpan {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: false,
            italic: false,
            underline: false,
            color: None,
            font: None,
            font_size: None,
        }
    }
    pub fn bold(mut self) -> Self { self.bold = true; self }
    pub fn italic(mut self) -> Self { self.italic = true; self }
    pub fn underline(mut self) -> Self { self.underline = true; self }
    pub fn color(mut self, c: ColorToken) -> Self { self.color = Some(c); self }
    pub fn font(mut self, f: FontToken) -> Self { self.font = Some(f); self }
    pub fn size(mut self, px: f32) -> Self { self.font_size = Some(px); self }
}

/// A single action in a `Toolbar`.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolbarAction {
    pub id:        String,
    pub label:     Option<String>,
    pub icon_name: Option<String>,
    pub handler:   HandlerId,
    pub separator: bool,
    pub enabled:   bool,
}

impl ToolbarAction {
    pub fn new(id: impl Into<String>, handler: HandlerId) -> Self {
        Self {
            id: id.into(),
            label: None,
            icon_name: None,
            handler,
            separator: false,
            enabled: true,
        }
    }
    pub fn label(mut self, l: impl Into<String>) -> Self { self.label = Some(l.into()); self }
    pub fn icon(mut self, name: impl Into<String>) -> Self { self.icon_name = Some(name.into()); self }
    pub fn separator(mut self) -> Self { self.separator = true; self }
    pub fn disabled(mut self) -> Self { self.enabled = false; self }
}

/// 2D point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// Edge insets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Insets {
    pub top:    f32,
    pub right:  f32,
    pub bottom: f32,
    pub left:   f32,
}

pub type Props = HashMap<PropKey, PropValue>;

// ---------------------------------------------------------------------------
// Changeset
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Operation {
    Create {
        parent_id: Option<NodeId>,
        node_id: NodeId,
        kind: ComponentKind,
        props: Props,
    },
    Update {
        node_id: NodeId,
        changed_props: Props,
    },
    Delete {
        node_id: NodeId,
    },
    Reorder {
        parent_id: NodeId,
        child_order: Vec<NodeId>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct Changeset {
    pub operations: Vec<Operation>,
}

impl Changeset {
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    pub fn summary(&self) -> String {
        let (mut c, mut u, mut d, mut r) = (0, 0, 0, 0);
        for op in &self.operations {
            match op {
                Operation::Create { .. } => c += 1,
                Operation::Update { .. } => u += 1,
                Operation::Delete { .. } => d += 1,
                Operation::Reorder { .. } => r += 1,
            }
        }
        format!("ops: create={c} update={u} delete={d} reorder={r}")
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    Press,
    Change,
    Close,
    Focus,
    Blur,
    KeyDown,
    KeyUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Char(char),
    Other(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventPayload {
    None,
    Text(String),
    Key(KeyCode),
    Float(f32),
    Index(i32),
    Color(ColorValue),
    Bool(bool),
    // Phase 2
    Scroll(ScrollEvent),
    Search(String),
    Segment(i32),
    SplitResize(Vec<f32>),
    ToolbarAction(String),
}

/// Scroll position and velocity delivered with scroll events.
#[derive(Debug, Clone, PartialEq)]
pub struct ScrollEvent {
    pub offset_x:   f32,
    pub offset_y:   f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub source: NodeId,
    pub handler: HandlerId,
    pub kind: EventKind,
    pub payload: EventPayload,
    /// When set to `true` by a consumer handler, prevents the event from
    /// propagating further up the component tree (FR-015).
    pub stop_bubbling: bool,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum RenderError {
    OsError(String),
    InvalidNode(NodeId),
    WrongThread,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::OsError(s) => write!(f, "os error: {s}"),
            RenderError::InvalidNode(n) => write!(f, "invalid node: {n:?}"),
            RenderError::WrongThread => write!(f, "renderer called from non-main thread"),
        }
    }
}

impl std::error::Error for RenderError {}

// ---------------------------------------------------------------------------
// PlatformRenderer trait
// ---------------------------------------------------------------------------

pub trait PlatformRenderer {
    fn apply(&mut self, changeset: Changeset) -> Result<(), RenderError>;
    fn run_event_loop(&mut self) -> Result<(), RenderError>;
}

// ---------------------------------------------------------------------------
// Dispatch bridge
// ---------------------------------------------------------------------------

type DispatchFn = fn(Event);

static DISPATCH: OnceLock<DispatchFn> = OnceLock::new();

#[doc(hidden)]
pub fn set_dispatch_bridge(f: DispatchFn) {
    let _ = DISPATCH.set(f);
}

pub fn dispatch_event(event: Event) {
    if let Some(f) = DISPATCH.get() {
        f(event);
    }
}

// ---------------------------------------------------------------------------
// Tick bridge
// ---------------------------------------------------------------------------

type TickFn = fn();

static TICK: OnceLock<TickFn> = OnceLock::new();

#[doc(hidden)]
pub fn set_tick_bridge(f: TickFn) {
    let _ = TICK.set(f);
}

pub fn tick() {
    if let Some(f) = TICK.get() {
        f()
    }
}

// ---------------------------------------------------------------------------
// Apply bridge
// ---------------------------------------------------------------------------

pub trait ApplyBridge: Send + Sync {
    fn apply(&self, changeset: Changeset) -> Result<(), RenderError>;
}

static APPLY: OnceLock<Box<dyn ApplyBridge>> = OnceLock::new();

#[doc(hidden)]
pub fn set_apply_bridge(bridge: Box<dyn ApplyBridge>) {
    let _ = APPLY.set(bridge);
}

pub fn apply_via_bridge(changeset: Changeset) -> Result<(), RenderError> {
    match APPLY.get() {
        Some(bridge) => bridge.apply(changeset),
        None => Err(RenderError::OsError(
            "no apply bridge installed (call rn::run first)".to_string(),
        )),
    }
}
