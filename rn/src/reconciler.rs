//! Reconciler: diffs two consecutive `ComponentTree` snapshots and produces a
//! minimal `Changeset`.
//!
//! The algorithm is React's O(n) heuristic: nodes at the same tree position
//! with the same kind are the same node. Keyed nodes allow explicit identity
//! across reorders. See research.md for details.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

pub use rn_core::{Changeset, Operation};

use rn_core::{NodeId, PropValue, Props};

use crate::component::{Component, ComponentKind, ComponentTree};
use crate::error::ReconcileError;

static NEXT_NODE_ID: AtomicU64 = AtomicU64::new(1);

fn alloc_node_id() -> NodeId {
    NodeId(NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed))
}

/// Internal shadow tree: an `AssignedNode` is the previously-rendered
/// `Component` plus its reconciler-assigned `NodeId`.
#[derive(Debug, Clone)]
struct AssignedNode {
    node_id: NodeId,
    kind: ComponentKind,
    props: Props,
    key: Option<String>,
    children: Vec<AssignedNode>,
}

impl AssignedNode {
    fn from_component(component: &Component) -> Self {
        Self {
            node_id: alloc_node_id(),
            kind: component.kind,
            props: component.props.clone(),
            key: component.key.clone(),
            children: component.children.iter().map(AssignedNode::from_component).collect(),
        }
    }
}

/// Stateful reconciler. Owns the previously rendered tree.
pub struct Reconciler {
    previous: Option<AssignedNode>,
}

impl Reconciler {
    pub fn new() -> Self {
        Self { previous: None }
    }

    /// Reconcile `next` against the previous tree. Returns the minimal
    /// changeset to apply.
    pub fn reconcile(&mut self, next: ComponentTree) -> Result<Changeset, ReconcileError> {
        let mut ops: Vec<Operation> = Vec::new();

        let new_root = match self.previous.take() {
            None => {
                let assigned = AssignedNode::from_component(&next.root);
                emit_create(&assigned, None, &mut ops);
                assigned
            }
            Some(prev) => diff_node(Some(prev), &next.root, None, &mut ops)?
                .ok_or(ReconcileError::OrphanedNode(NodeId::UNASSIGNED))?,
        };

        self.previous = Some(new_root);
        Ok(Changeset { operations: ops })
    }
}

impl Default for Reconciler {
    fn default() -> Self {
        Self::new()
    }
}

fn emit_create(node: &AssignedNode, parent_id: Option<NodeId>, ops: &mut Vec<Operation>) {
    ops.push(Operation::Create {
        parent_id,
        node_id: node.node_id,
        kind: node.kind,
        props: node.props.clone(),
    });
    for child in &node.children {
        emit_create(child, Some(node.node_id), ops);
    }
}

fn emit_delete(node: &AssignedNode, ops: &mut Vec<Operation>) {
    for child in &node.children {
        emit_delete(child, ops);
    }
    ops.push(Operation::Delete { node_id: node.node_id });
}

fn diff_node(
    prev: Option<AssignedNode>,
    next: &Component,
    parent_id: Option<NodeId>,
    ops: &mut Vec<Operation>,
) -> Result<Option<AssignedNode>, ReconcileError> {
    match prev {
        None => {
            let assigned = AssignedNode::from_component(next);
            emit_create(&assigned, parent_id, ops);
            Ok(Some(assigned))
        }
        Some(prev_node) => {
            if prev_node.kind != next.kind {
                emit_delete(&prev_node, ops);
                let assigned = AssignedNode::from_component(next);
                emit_create(&assigned, parent_id, ops);
                return Ok(Some(assigned));
            }

            let changed = diff_props(&prev_node.props, &next.props);
            if !changed.is_empty() {
                ops.push(Operation::Update {
                    node_id: prev_node.node_id,
                    changed_props: changed,
                });
            }

            let new_children = diff_children(prev_node.children, &next.children, prev_node.node_id, ops)?;

            Ok(Some(AssignedNode {
                node_id: prev_node.node_id,
                kind: next.kind,
                props: next.props.clone(),
                key: next.key.clone(),
                children: new_children,
            }))
        }
    }
}

fn diff_props(old: &Props, new: &Props) -> Props {
    let mut changed: Props = Props::new();
    for (key, new_val) in new {
        match old.get(key) {
            Some(old_val) if props_equal(old_val, new_val) => {}
            _ => {
                changed.insert(*key, new_val.clone());
            }
        }
    }
    for (key, old_val) in old {
        if !new.contains_key(key) {
            changed.insert(*key, prop_reset_value(old_val));
        }
    }
    changed
}

fn props_equal(a: &PropValue, b: &PropValue) -> bool {
    match (a, b) {
        (PropValue::String(x), PropValue::String(y)) => x == y,
        (PropValue::Bool(x), PropValue::Bool(y)) => x == y,
        (PropValue::F32(x), PropValue::F32(y)) => x.to_bits() == y.to_bits(),
        (PropValue::HandlerRef(x), PropValue::HandlerRef(y)) => x == y,
        (PropValue::StringList(x), PropValue::StringList(y)) => x == y,
        (PropValue::Color(x), PropValue::Color(y)) => x == y,
        _ => false,
    }
}

fn prop_reset_value(reference: &PropValue) -> PropValue {
    match reference {
        PropValue::String(_) => PropValue::String(String::new()),
        PropValue::Bool(_) => PropValue::Bool(false),
        PropValue::F32(_) => PropValue::F32(0.0),
        PropValue::HandlerRef(id) => PropValue::HandlerRef(*id),
        PropValue::StringList(_) => PropValue::StringList(Vec::new()),
        PropValue::Color(_) => PropValue::Color(rn_core::ColorValue::rgb(0, 0, 0)),
        // Phase 2 — return a clone of the reference as a reasonable reset
        PropValue::NodeRef(id) => PropValue::NodeRef(*id),
        PropValue::Spans(_) => PropValue::Spans(Vec::new()),
        PropValue::ToolbarActions(_) => PropValue::ToolbarActions(Vec::new()),
        PropValue::F32List(_) => PropValue::F32List(Vec::new()),
        PropValue::ColorToken(t) => PropValue::ColorToken(*t),
        PropValue::SpacingToken(t) => PropValue::SpacingToken(*t),
        PropValue::RadiusToken(t) => PropValue::RadiusToken(*t),
        PropValue::FontToken(t) => PropValue::FontToken(*t),
    }
}

fn diff_children(
    prev: Vec<AssignedNode>,
    next: &[Component],
    parent_id: NodeId,
    ops: &mut Vec<Operation>,
) -> Result<Vec<AssignedNode>, ReconcileError> {
    // Enforce key uniqueness per parent.
    {
        let mut seen = std::collections::HashSet::new();
        for c in next {
            if let Some(k) = &c.key {
                if !seen.insert(k.clone()) {
                    return Err(ReconcileError::DuplicateKey(k.clone()));
                }
            }
        }
    }

    // Capture the previous order (by node_id) BEFORE moving nodes into the
    // key/unkeyed maps. Keyed matches will consume these out of order; we
    // compare the surviving new order against this prev_order to detect
    // reorders.
    let prev_order_full: Vec<NodeId> = prev.iter().map(|p| p.node_id).collect();

    let mut prev_by_key: HashMap<String, AssignedNode> = HashMap::new();
    let mut prev_unkeyed: Vec<AssignedNode> = Vec::new();
    for p in prev {
        if let Some(k) = &p.key {
            prev_by_key.insert(k.clone(), p);
        } else {
            prev_unkeyed.push(p);
        }
    }
    prev_unkeyed.reverse();

    let mut new_children: Vec<AssignedNode> = Vec::with_capacity(next.len());

    for next_child in next {
        let matched = if let Some(k) = &next_child.key {
            prev_by_key.remove(k)
        } else {
            prev_unkeyed.pop()
        };

        let assigned = diff_node(matched, next_child, Some(parent_id), ops)?
            .ok_or(ReconcileError::OrphanedNode(NodeId::UNASSIGNED))?;
        new_children.push(assigned);
    }

    for (_, orphan) in prev_by_key.drain() {
        emit_delete(&orphan, ops);
    }
    for orphan in prev_unkeyed.drain(..) {
        emit_delete(&orphan, ops);
    }

    // Reorder detection: among nodes that survived (appear in both prev and
    // new), does the new order match the previous order?
    let new_order: Vec<NodeId> = new_children.iter().map(|c| c.node_id).collect();
    let survived_prev: Vec<NodeId> = prev_order_full
        .iter()
        .copied()
        .filter(|id| new_order.contains(id))
        .collect();
    let survived_new: Vec<NodeId> = new_order
        .iter()
        .copied()
        .filter(|id| survived_prev.contains(id))
        .collect();
    if !survived_prev.is_empty() && survived_prev != survived_new {
        ops.push(Operation::Reorder {
            parent_id,
            child_order: new_order,
        });
    }

    Ok(new_children)
}
