// rn-macos/src/tree.rs — Phase A: NSOutlineView tree component

use std::cell::RefCell;
use std::rc::Rc;
use objc2::rc::Retained;
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{
    NSControlTextEditingDelegate,
    NSOutlineView, NSOutlineViewDataSource, NSOutlineViewDelegate,
    NSTableCellView, NSTextField, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSInteger, NSNotification, NSNumber, NSObject,
    NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};
use rn_core::{
    data::{RowId, TreeModel, TreeNode},
    Event, EventKind, EventPayload, HandlerId, NodeId,
};

// Encode a node as an NSNumber index into a flat pre-order snapshot so we can
// hand NSOutlineView stable item pointers without keeping NSObject nodes.
//
// The snapshot is rebuilt every time the model changes.
#[derive(Clone)]
#[allow(dead_code)]
struct FlatNode {
    id: RowId,
    label: String,
    icon: Option<String>,
    has_children: bool,
    expanded: bool,
    n_children: usize,
    // index of first child in the flat list, or usize::MAX if leaf
    first_child: usize,
    // index of parent, or usize::MAX if root
    parent: usize,
}

type Snapshot = Rc<Vec<FlatNode>>;

fn build_snapshot(model: &TreeModel) -> Snapshot {
    let mut out: Vec<FlatNode> = Vec::new();
    append_nodes(&model.roots, usize::MAX, &mut out);
    Rc::new(out)
}

fn append_nodes(nodes: &[TreeNode], parent: usize, out: &mut Vec<FlatNode>) {
    for node in nodes {
        let my_idx = out.len();
        let first_child = if node.has_children() { my_idx + 1 } else { usize::MAX };
        out.push(FlatNode {
            id: node.id,
            label: node.label.clone(),
            icon: node.icon.clone(),
            has_children: node.has_children(),
            expanded: node.expanded,
            n_children: node.children.len(),
            first_child,
            parent,
        });
        if node.has_children() {
            append_nodes(&node.children, my_idx, out);
        }
    }
}

// Roots are nodes whose parent == usize::MAX.
fn roots(snap: &[FlatNode]) -> Vec<usize> {
    snap.iter().enumerate()
        .filter(|(_, n)| n.parent == usize::MAX)
        .map(|(i, _)| i)
        .collect()
}

fn children_of(snap: &[FlatNode], idx: usize) -> Vec<usize> {
    snap.iter().enumerate()
        .filter(|(_, n)| n.parent == idx)
        .map(|(i, _)| i)
        .collect()
}

// We pass item indices as NSNumber(i64) to NSOutlineView.
fn idx_from_item(item: Option<&objc2::runtime::AnyObject>) -> Option<usize> {
    let item = item?;
    let num: &NSNumber = unsafe { &*(item as *const _ as *const NSNumber) };
    let v = num.longLongValue();
    if v < 0 { None } else { Some(v as usize) }
}

fn idx_to_item(_mtm: MainThreadMarker, idx: usize) -> Retained<NSNumber> {
    NSNumber::numberWithLongLong(idx as i64)
}

// ---------------------------------------------------------------------------
// Shared inner
// ---------------------------------------------------------------------------
pub(crate) struct TreeSourcesInner {
    pub(crate) model: RefCell<TreeModel>,
    snapshot: RefCell<Snapshot>,
    /// Tracks which RowIds are currently expanded in the outline view.
    /// Updated on expand/collapse events. Used by restore_expansion after reloadData.
    pub(crate) expanded_ids: RefCell<std::collections::HashSet<RowId>>,
}

impl TreeSourcesInner {
    fn new(model: TreeModel) -> Self {
        let snapshot = build_snapshot(&model);
        // Seed expanded_ids from model's initial expanded state.
        let expanded_ids: std::collections::HashSet<RowId> =
            snapshot.iter().filter(|n| n.expanded).map(|n| n.id).collect();
        TreeSourcesInner {
            model: RefCell::new(model),
            snapshot: RefCell::new(snapshot),
            expanded_ids: RefCell::new(expanded_ids),
        }
    }
    fn refresh_snapshot(&self) {
        *self.snapshot.borrow_mut() = build_snapshot(&self.model.borrow());
    }
}

/// Holds data source + delegate pair; stash in DelegateHandle::Tree.
pub(crate) struct TreeSources {
    pub data: Retained<TreeDataSource>,
    pub delegate: Retained<TreeDelegate>,
    #[allow(dead_code)]
    pub inner: Rc<TreeSourcesInner>,
    /// Optional target for NSOutlineView doubleAction (OnActivate).
    pub activate: Option<Retained<objc2::runtime::NSObject>>,
}

// ---------------------------------------------------------------------------
// Data source
// ---------------------------------------------------------------------------

pub(crate) struct DataIvars {
    inner: *const TreeSourcesInner,
    #[allow(dead_code)] source: NodeId,
}


unsafe fn get_child_item(
    ds: &TreeDataSource,
    child: NSInteger,
    item: Option<&objc2::runtime::AnyObject>,
) -> Option<Retained<NSObject>> {
    let mtm = MainThreadMarker::new()?;
    let inner = &*ds.ivars().inner;
    let snap = inner.snapshot.borrow();
    let idx = match idx_from_item(item) {
        None => roots(&snap).get(child as usize).copied(),
        Some(parent) => children_of(&snap, parent).get(child as usize).copied(),
    }?;
    Some(Retained::cast(idx_to_item(mtm, idx)))
}

declare_class!(
    pub(crate) struct TreeDataSource;

    unsafe impl ClassType for TreeDataSource {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "RnTreeDataSource";
    }

    impl DeclaredClass for TreeDataSource {
        type Ivars = DataIvars;
    }

    unsafe impl NSObjectProtocol for TreeDataSource {}

    unsafe impl NSOutlineViewDataSource for TreeDataSource {
        #[method(outlineView:numberOfChildrenOfItem:)]
        unsafe fn number_of_children(
            &self,
            _ov: &NSOutlineView,
            item: Option<&objc2::runtime::AnyObject>,
        ) -> NSInteger {
            let inner = &*self.ivars().inner;
            let snap = inner.snapshot.borrow();
            match idx_from_item(item) {
                None => roots(&snap).len() as NSInteger,
                Some(idx) => children_of(&snap, idx).len() as NSInteger,
            }
        }

        #[method_id(outlineView:child:ofItem:)]
        unsafe fn child_of_item(
            &self,
            _ov: &NSOutlineView,
            child: NSInteger,
            item: Option<&objc2::runtime::AnyObject>,
        ) -> Option<Retained<NSObject>> {
            get_child_item(self, child, item)
        }

        #[method(outlineView:isItemExpandable:)]
        unsafe fn is_expandable(
            &self,
            _ov: &NSOutlineView,
            item: &objc2::runtime::AnyObject,
        ) -> bool {
            let inner = &*self.ivars().inner;
            let snap = inner.snapshot.borrow();
            match idx_from_item(Some(item)) {
                Some(i) => snap.get(i).is_some_and(|n| n.has_children),
                None => false,
            }
        }
    }
);

impl TreeDataSource {
    fn new(mtm: MainThreadMarker, inner: *const TreeSourcesInner, source: NodeId) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(DataIvars { inner, source });
        unsafe { msg_send_id![super(this), init] }
    }
}

// ---------------------------------------------------------------------------
// Delegate
// ---------------------------------------------------------------------------

pub(crate) struct DelegateIvars {
    inner: *const TreeSourcesInner,
    source: NodeId,
    handler_select: Option<HandlerId>,
    #[allow(dead_code)] handler_activate: Option<HandlerId>,
}

unsafe fn make_tree_cell(
    delegate: &TreeDelegate,
    item: &objc2::runtime::AnyObject,
) -> Option<Retained<NSView>> {
    let inner = &*delegate.ivars().inner;
    let snap = inner.snapshot.borrow();
    let idx = idx_from_item(Some(item))?;
    let node = snap.get(idx)?;
    let label = node.label.clone();
    let icon = node.icon.clone();
    drop(snap); // release borrow before MainThreadMarker
    let mtm = MainThreadMarker::new()?;
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0));
    let tcv = NSTableCellView::initWithFrame(mtm.alloc(), frame);
    // Add icon if present.
    if let Some(icon_name) = icon {
        let img = objc2_app_kit::NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str(&icon_name), None,
        );
        if let Some(img) = img {
            let icon_frame = NSRect::new(NSPoint::new(2.0, 2.0), NSSize::new(14.0, 14.0));
            let iv = objc2_app_kit::NSImageView::initWithFrame(mtm.alloc(), icon_frame);
            iv.setImage(Some(&img));
            tcv.addSubview(&iv);
        }
    }
    let tf = NSTextField::labelWithString(&NSString::from_str(&label), mtm);
    tf.setEditable(false);
    tf.setBordered(false);
    tf.setDrawsBackground(false);
    tcv.setTextField(Some(&tf));
    tcv.addSubview(&tf);
    Some(Retained::cast(tcv))
}


/// Keep `expanded_ids` in sync when the user interactively expands/collapses a row.
/// This lets restore_expansion round-trip correctly after a model refresh (FR-016).
unsafe fn set_item_expanded(delegate: &TreeDelegate, notification: &NSNotification, expanded: bool) {
    // The notification userInfo has "NSObject" -> the item. Decode the tag.
    // Simplest: ask the outline view which row is selected / just changed.
    // We use the approach of reading the item from the notification directly.
    let inner = &*delegate.ivars().inner;
    // Walk userInfo to get the item.
    let user_info = match notification.userInfo() { Some(u) => u, None => return };
    let key = NSString::from_str("NSObject");
    let item_obj = match user_info.objectForKey(&*key) { Some(o) => o, None => return };
    let num: &NSNumber = &*(Retained::as_ptr(&item_obj) as *const NSNumber);
    let idx = num.longLongValue();
    if idx < 0 { return; }
    let snap = inner.snapshot.borrow();
    if let Some(node) = snap.get(idx as usize) {
        let id = node.id;
        drop(snap);
        if expanded {
            inner.expanded_ids.borrow_mut().insert(id);
        } else {
            inner.expanded_ids.borrow_mut().remove(&id);
        }
    }
}

declare_class!(
    pub(crate) struct TreeDelegate;

    unsafe impl ClassType for TreeDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "RnTreeDelegate";
    }

    impl DeclaredClass for TreeDelegate {
        type Ivars = DelegateIvars;
    }

    unsafe impl NSObjectProtocol for TreeDelegate {}
    unsafe impl NSControlTextEditingDelegate for TreeDelegate {}

    unsafe impl NSOutlineViewDelegate for TreeDelegate {
        #[method_id(outlineView:viewForTableColumn:item:)]
        unsafe fn view_for_item(
            &self,
            _ov: &NSOutlineView,
            _col: Option<&objc2_app_kit::NSTableColumn>,
            item: &objc2::runtime::AnyObject,
        ) -> Option<Retained<NSView>> {
            make_tree_cell(self, item)
        }


        #[method(outlineViewItemDidExpand:)]
        unsafe fn item_did_expand(&self, notification: &NSNotification) {
            set_item_expanded(self, notification, true);
        }

        #[method(outlineViewItemDidCollapse:)]
        unsafe fn item_did_collapse(&self, notification: &NSNotification) {
            set_item_expanded(self, notification, false);
        }
        #[method(outlineViewSelectionDidChange:)]
        unsafe fn selection_changed(&self, notification: &NSNotification) {
            let ov = match notification.object() { Some(o) => o, None => return };
            let ov: &NSOutlineView = &*(Retained::as_ptr(&ov) as *const NSOutlineView);
            let row = ov.selectedRow();
            if row < 0 {
                // Deselect: clear selection in model.
                let inner = &*self.ivars().inner;
                inner.model.borrow_mut().selection = None;
                return;
            }
            // FR-016/US3: write selected RowId back to model so it survives refresh.
            let inner = &*self.ivars().inner;
            {
                let snap = inner.snapshot.borrow();
                // The selected outline row maps to a snapshot index via itemAtRow.
                let item = ov.itemAtRow(row);
                if let Some(item) = item {
                    let num: &NSNumber = &*(Retained::as_ptr(&item) as *const NSNumber);
                    let idx = num.longLongValue();
                    if idx >= 0 {
                        if let Some(node) = snap.get(idx as usize) {
                            inner.model.borrow_mut().selection = Some(node.id);
                        }
                    }
                }
            }
            if let Some(h) = self.ivars().handler_select {
                rn_core::dispatch_event(Event {
                    source: self.ivars().source,
                    handler: h,
                    kind: EventKind::Press,
                    payload: EventPayload::None,
                    stop_bubbling: false,
                });
            }
        }
    }
);

impl TreeDelegate {
    fn new(
        mtm: MainThreadMarker,
        inner: *const TreeSourcesInner,
        source: NodeId,
        handler_select: Option<HandlerId>,
        handler_activate: Option<HandlerId>,
    ) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(DelegateIvars {
            inner, source, handler_select, handler_activate,
        });
        unsafe { msg_send_id![super(this), init] }
    }
}

// ---------------------------------------------------------------------------
// Public constructor
// ---------------------------------------------------------------------------


/// After `reloadData()`, expand all rows whose node id is in `ids`.
/// Takes the current snapshot so it can map outline-view item tags back to RowIds.
/// Called from `update_node` to restore expansion state across a model refresh (FR-016).
pub(crate) fn restore_expansion(ov: &NSOutlineView, sources: &TreeSources, ids: &[RowId]) {
    if ids.is_empty() { return; }
    let snap = sources.inner.snapshot.borrow();
    let n = unsafe { ov.numberOfRows() };
    for row in 0..n {
        let item = unsafe { ov.itemAtRow(row) };
        if let Some(item) = item {
            let num: &NSNumber = unsafe { &*(Retained::as_ptr(&item) as *const NSNumber) };
            let idx = num.longLongValue();
            if idx >= 0 {
                if let Some(node) = snap.get(idx as usize) {
                    if ids.contains(&node.id) {
                        unsafe { ov.expandItem(Some(&*item)); }
                    }
                }
            }
        }
    }
}


/// Compute the maximum depth of any node in the snapshot (root = depth 0).
#[allow(dead_code)]
fn max_depth_in_snapshot(snap: &[FlatNode]) -> usize {
    let mut depths: Vec<usize> = vec![0; snap.len()];
    for (i, node) in snap.iter().enumerate() {
        if node.parent != usize::MAX {
            depths[i] = depths[node.parent] + 1;
        }
    }
    depths.into_iter().max().unwrap_or(0)
}

/// Compute max depth directly from TreeNode roots (does not need a built snapshot).
pub(crate) fn model_max_depth(nodes: &[rn_core::data::TreeNode]) -> usize {
    fn depth(node: &rn_core::data::TreeNode, current: usize) -> usize {
        if node.children.is_empty() { current }
        else { node.children.iter().map(|c| depth(c, current + 1)).max().unwrap_or(current) }
    }
    nodes.iter().map(|n| depth(n, 0)).max().unwrap_or(0)
}


pub(crate) fn make_tree_sources(
    mtm: MainThreadMarker,
    model: TreeModel,
    source: NodeId,
    handler_select: Option<HandlerId>,
    handler_activate: Option<HandlerId>,
) -> TreeSources {
    let inner = Rc::new(TreeSourcesInner::new(model));
    let raw: *const TreeSourcesInner = Rc::as_ptr(&inner);
    let data = TreeDataSource::new(mtm, raw, source);
    let delegate = TreeDelegate::new(mtm, raw, source, handler_select, handler_activate);
    TreeSources { data, delegate, inner, activate: None }
}

#[allow(dead_code)]
pub(crate) fn update_tree_model(sources: &TreeSources, new_model: TreeModel) {
    *sources.inner.model.borrow_mut() = new_model;
    sources.inner.refresh_snapshot();
}
