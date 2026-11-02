//! Core types shared between `rn` and its platform backend crates.
//!
//! This crate exists so backend crates can implement the `PlatformRenderer`
//! trait without introducing a circular dependency on the top-level `rn`
//! crate.

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropKey {
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
    OnPress,
    OnChange,
    OnClose,
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

#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    String(String),
    Bool(bool),
    F32(f32),
    HandlerRef(HandlerId),
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
// PlatformRenderer trait — the single seam between framework and backends
// ---------------------------------------------------------------------------

pub trait PlatformRenderer {
    fn apply(&mut self, changeset: Changeset) -> Result<(), RenderError>;
    fn run_event_loop(&mut self) -> Result<(), RenderError>;
}

// ---------------------------------------------------------------------------
// Dispatch bridge — lets a backend crate (which depends only on rn-core) hand
// native-UI events to the top-level `rn` crate's handler registry without a
// circular dependency. `rn` registers its `dispatch_handler` at startup via
// `set_dispatch_bridge`; backends call `dispatch_event` on each native event.
// ---------------------------------------------------------------------------

type DispatchFn = fn(Event);

static DISPATCH: OnceLock<DispatchFn> = OnceLock::new();

/// Install the event dispatcher. Called once by `rn` during initialisation;
/// idempotent — the first registered function wins.
pub fn set_dispatch_bridge(f: DispatchFn) {
    let _ = DISPATCH.set(f);
}

/// Backends call this to hand a native event to the framework. If no bridge
/// has been installed yet (e.g. tests without `rn`), the event is dropped.
pub fn dispatch_event(event: Event) {
    if let Some(f) = DISPATCH.get() {
        f(event);
    }
}

// ---------------------------------------------------------------------------
// Tick bridge — lets a backend crate drain the host's pending state updates
// from `rn::drain_updates()` once per event-loop tick without depending on
// `rn`. `rn::run` installs a trampoline that calls back into the host closure.
// ---------------------------------------------------------------------------

type TickFn = fn();

static TICK: OnceLock<TickFn> = OnceLock::new();

/// Install the per-tick trampoline. Idempotent.
pub fn set_tick_bridge(f: TickFn) {
    let _ = TICK.set(f);
}

/// Backends invoke this from their event loop timer; a no-op if nothing is
/// registered yet.
pub fn tick() {
    if let Some(f) = TICK.get() {
        f();
    }
}

// ---------------------------------------------------------------------------
// Apply bridge — the backend owns the renderer's native handles behind
// `&mut self`, but re-renders can originate from the event loop tick where
// the renderer is already borrowed by the event-loop driver. The backend
// installs an `ApplyBridge` that funnels a `Changeset` back into its internal
// state through interior mutability; `rn::rerender` uses this to apply
// changesets produced by the per-tick view function.
// ---------------------------------------------------------------------------

pub trait ApplyBridge: Send + Sync {
    fn apply(&self, changeset: Changeset) -> Result<(), RenderError>;
}

static APPLY: OnceLock<Box<dyn ApplyBridge>> = OnceLock::new();

/// Install the apply bridge. Idempotent — first registration wins.
pub fn set_apply_bridge(bridge: Box<dyn ApplyBridge>) {
    let _ = APPLY.set(bridge);
}

/// Framework-internal: apply a changeset via whatever bridge the backend
/// installed. Returns `Err(RenderError::OsError(..))` if no bridge is
/// installed.
pub fn apply_via_bridge(changeset: Changeset) -> Result<(), RenderError> {
    match APPLY.get() {
        Some(bridge) => bridge.apply(changeset),
        None => Err(RenderError::OsError(
            "no apply bridge installed (call rn::run first)".to_string(),
        )),
    }
}
