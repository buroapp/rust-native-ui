//! Error types. `RenderError` is shared with backends via `rn-core`;
//! `ReconcileError` and `RnError` are `rn`-only (backends never run the
//! reconciler).

pub use rn_core::RenderError;

use rn_core::NodeId;

#[derive(Debug, Clone)]
pub enum ReconcileError {
    DuplicateKey(String),
    OrphanedNode(NodeId),
}

#[derive(Debug, Clone)]
pub enum RnError {
    Render(RenderError),
    Reconcile(ReconcileError),
    WrongThread,
}

impl std::fmt::Display for ReconcileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReconcileError::DuplicateKey(k) => write!(f, "duplicate reconciler key: {k}"),
            ReconcileError::OrphanedNode(n) => write!(f, "orphaned node: {n:?}"),
        }
    }
}

impl std::fmt::Display for RnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RnError::Render(e) => write!(f, "{e}"),
            RnError::Reconcile(e) => write!(f, "{e}"),
            RnError::WrongThread => write!(f, "rn::render/rn::run called from non-main thread"),
        }
    }
}

impl std::error::Error for ReconcileError {}
impl std::error::Error for RnError {}
