//! Core types shared between `rn` and its platform backend crates.
//!
//! This crate exists so backend crates can implement the `PlatformRenderer`
//! trait without introducing a circular dependency on the top-level `rn`
//! crate.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

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
