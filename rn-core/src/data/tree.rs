use super::RowId;

/// One node in a tree. Children are owned by value, preventing cycles.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeNode {
    pub id: RowId,
    pub label: String,
    pub icon: Option<String>,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
}

impl TreeNode {
    pub fn leaf(id: u64, label: impl Into<String>) -> Self {
        TreeNode { id: RowId(id), label: label.into(), icon: None, children: vec![], expanded: false }
    }
    pub fn branch(id: u64, label: impl Into<String>, children: Vec<TreeNode>) -> Self {
        TreeNode { id: RowId(id), label: label.into(), icon: None, children, expanded: false }
    }
    pub fn icon(mut self, icon: impl Into<String>) -> Self { self.icon = Some(icon.into()); self }
    pub fn expanded(mut self) -> Self { self.expanded = true; self }
    pub fn has_children(&self) -> bool { !self.children.is_empty() }
    /// Collect ids of all expanded nodes in pre-order.
    pub fn collect_expanded(&self, out: &mut Vec<RowId>) {
        if self.expanded { out.push(self.id); }
        for c in &self.children { c.collect_expanded(out); }
    }
}

/// The full tree data model.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TreeModel {
    pub roots: Vec<TreeNode>,
    pub selection: Option<RowId>,
}

impl TreeModel {
    pub fn new(roots: Vec<TreeNode>) -> Self { TreeModel { roots, selection: None } }
    /// Collect the ids of all expanded nodes.
    pub fn find_expanded(&self) -> Vec<RowId> {
        let mut out = Vec::new();
        for r in &self.roots { r.collect_expanded(&mut out); }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn expanded_ids_collected() {
        let node = TreeNode::branch(1, "parent", vec![TreeNode::leaf(2, "child")]).expanded();
        let m = TreeModel::new(vec![node]);
        assert_eq!(m.find_expanded(), vec![RowId(1)]);
    }
    #[test] fn leaf_has_no_children() {
        assert!(!TreeNode::leaf(0, "x").has_children());
    }
}
