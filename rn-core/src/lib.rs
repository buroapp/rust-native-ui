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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentKind {
    Window,
    Label,
    Button,
    TextInput,
    LayoutContainer(Axis),
    // New in v0.2
    Checkbox,        // NSButton (switch style) — Bool state
    Slider,          // NSSlider — F32 value in [min, max]
    ProgressBar,     // NSProgressIndicator (determinate)
    Separator,       // NSBox separator
    TextArea,        // NSTextView (multi-line, scrolled)
    Dropdown,        // NSPopUpButton — index into StringList options
    Image,           // NSImageView — SF Symbol / named image
    Stepper,         // NSStepper + value label
    ColorWell,       // NSColorWell — RGBA via ColorValue
    DatePicker,      // NSDatePicker — date string (ISO 8601)
    RadioGroup,      // NSMatrix of NSButtonCell (radio) — index-based
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
    Title,
    Placeholder,
    // Handlers
    OnPress,
    OnChange,
    OnClose,
    // New: numeric range & value
    Value,       // F32 — slider/progress/stepper current value
    Min,         // F32
    Max,         // F32
    Step,        // F32 — stepper increment
    // New: boolean state
    Checked,     // Bool — checkbox / radio checked state
    // New: selection
    SelectedIndex, // F32 (cast to i32) — dropdown / radio group
    Options,       // StringList — dropdown / radio group items
    // New: media / appearance
    ImageName,   // String — SF Symbol name for NSImageView
    // New: color
    Color,       // ColorValue (r, g, b, a) × 255
    // New: date
    DateValue,   // String (ISO 8601: "YYYY-MM-DD")
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

#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    String(String),
    Bool(bool),
    F32(f32),
    HandlerRef(HandlerId),
    StringList(Vec<String>),  // options list for Dropdown / RadioGroup
    Color(ColorValue),
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
    Float(f32),           // slider, stepper, progress value
    Index(i32),           // dropdown / radio group selected index
    Color(ColorValue),    // color well selection
}

#[derive(Debug, Clone)]
pub struct Event {
    pub source: NodeId,
    pub handler: HandlerId,
    pub kind: EventKind,
    pub payload: EventPayload,
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
