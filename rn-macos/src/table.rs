// rn-macos/src/table.rs — Phase A: NSTableView data source and delegate

use std::cell::RefCell;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{
    NSControlTextEditingDelegate,
    NSTableCellView, NSTableColumn,
    NSTableView, NSTableViewDataSource, NSTableViewDelegate,
    NSTextField, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSInteger, NSNotification, NSObject,
    NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};
use rn_core::{
    data::{SortDescriptor, SortDirection, TableModel},
    Event, EventKind, EventPayload, HandlerId, NodeId,
};

// ---------------------------------------------------------------------------
// Shared model cell (guarded by RefCell so data source and delegate can both
// read from the same model without unsafe aliasing).
// ---------------------------------------------------------------------------
type ModelCell = RefCell<TableModel>;

// ---------------------------------------------------------------------------
// Data source
// ---------------------------------------------------------------------------

// NSObject is the simplest shareable container; we store the model pointer as
// a raw *const ModelCell to avoid lifetime issues inside declare_class!.
// Safety: the ModelCell outlives the data source and delegate because TableSources
// owns both and keeps the Arc alive.
use std::rc::Rc;

// TableSourcesInner is single-threaded (MainThreadOnly); Rc is sufficient.
pub(crate) struct TableSourcesInner {
    pub(crate) model: Rc<ModelCell>,
}

/// Holds a data source and delegate pair that share one `TableModel`.
/// Stash via `DelegateHandle::Table(table_sources)` to keep them alive.
pub(crate) struct TableSources {
    pub data: Retained<TableDataSource>,
    pub delegate: Retained<TableDelegate>,
    #[allow(dead_code)]
    pub(crate) inner: Rc<TableSourcesInner>,
    /// Optional target for the NSTableView doubleAction (OnActivate).
    pub activate: Option<Retained<objc2::runtime::NSObject>>,
}

// ---------------------------------------------------------------------------
// Data source: supplies row count and sort notification.
// ---------------------------------------------------------------------------

pub(crate) struct DataIvars {
    inner: *const TableSourcesInner,  // kept alive by TableSources.inner Arc
    #[allow(dead_code)] source: NodeId,
}

declare_class!(
    pub(crate) struct TableDataSource;

    unsafe impl ClassType for TableDataSource {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "RnTableDataSource";
    }

    impl DeclaredClass for TableDataSource {
        type Ivars = DataIvars;
    }

    unsafe impl NSObjectProtocol for TableDataSource {}

    unsafe impl NSTableViewDataSource for TableDataSource {
        #[method(numberOfRowsInTableView:)]
        unsafe fn number_of_rows(&self, _tv: &NSTableView) -> NSInteger {
            let inner = &*self.ivars().inner;
            inner.model.borrow().rows.len() as NSInteger
        }

        // Sort descriptor change: fired when the user clicks a column header.
        // We derive the new sort from the updated descriptors and dispatch it.
        #[method(tableView:sortDescriptorsDidChange:)]
        unsafe fn sort_changed(&self, tv: &NSTableView, _old: &AnyObject) {
            // The outline simply toggles direction per column; our model does the same.
            // Read the current leading sort descriptor name (we use column ColumnId as title).
            let inner = &*self.ivars().inner;
            let mut model = inner.model.borrow_mut();
            // NSTableView will have reordered its sort descriptors; read the first one.
            let descs = tv.sortDescriptors();
            let new_sort = if descs.count() == 0 {
                None
            } else {
                let desc = descs.objectAtIndex(0);
                let key_obj: Retained<AnyObject> = msg_send_id![&*desc, key];
                let key_str: &NSString = &*(Retained::as_ptr(&key_obj) as *const NSString);
                let col_title = key_str.to_string();
                model.columns.iter().find(|c| c.title == col_title).map(|c| {
                    let dir = match &model.sort {
                        Some(s) if s.column == c.id => s.direction.flip(),
                        _ => SortDirection::Ascending,
                    };
                    SortDescriptor { column: c.id, direction: dir }
                })
            };
            // FR-010: update sort indicator — must use new_sort (just computed), not model.sort (old value).
            if let Some(ref sd) = new_sort {
                // Find the NSTableColumn with a matching title and highlight it.
                let cols = tv.tableColumns();
                let n = cols.count();
                for i in 0..n {
                    let col = cols.objectAtIndex(i);
                    let title = col.identifier().to_string();
                    if let Some(my_col) = model.columns.iter().find(|c| c.id == sd.column) {
                        if title == my_col.title {
                            tv.setHighlightedTableColumn(Some(&col));
                            break;
                        }
                    }
                }
            } else {
                tv.setHighlightedTableColumn(None);
            }
            model.sort = new_sort.clone();
            // FR-010: physically reorder the rows so viewForTableColumn:row: returns
            // sorted data. Stable sort preserves relative order for equal-text rows.
            if let Some(ref sd) = new_sort {
                let col_index = model.columns.iter().position(|c| c.id == sd.column);
                if let Some(ci) = col_index {
                    let ascending = sd.direction == SortDirection::Ascending;
                    model.rows.sort_by(|a, b| {
                        let ta = a.cells.get(ci).map_or("", |c| c.text.as_str());
                        let tb = b.cells.get(ci).map_or("", |c| c.text.as_str());
                        if ascending { ta.cmp(tb) } else { tb.cmp(ta) }
                    });
                }
            }
            // FR-010/edge: preserve scroll position across sort reload.
            let first_visible = {
                let vis = tv.visibleRect();
                unsafe { tv.rowAtPoint(vis.origin) }
            };
            tv.reloadData();
            let row_count = unsafe { tv.numberOfRows() };
            if row_count > 0 && first_visible >= 0 {
                let target = first_visible.min(row_count - 1);
                unsafe { tv.scrollRowToVisible(target) };
            }
        }
    }
);

impl TableDataSource {
    fn new(mtm: MainThreadMarker, inner: *const TableSourcesInner, source: NodeId) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(DataIvars { inner, source });
        unsafe { msg_send_id![super(this), init] }
    }
}

// ---------------------------------------------------------------------------
// Delegate: supplies cell views and selection events.
// ---------------------------------------------------------------------------

pub(crate) struct DelegateIvars {
    inner: *const TableSourcesInner,
    source: NodeId,
    handler_select: Option<HandlerId>,
    #[allow(dead_code)] handler_activate: Option<HandlerId>,
}

// The declare_class! macro doesn't support ? inside method bodies, so make_cell_view
// uses match/return-None patterns instead. Suppress the question_mark lint here.
#[allow(clippy::question_mark)]
unsafe fn make_cell_view(
    delegate: &TableDelegate,
    col: Option<&NSTableColumn>,
    row: NSInteger,
) -> Option<Retained<NSView>> {
    let inner = &*delegate.ivars().inner;
    let model = inner.model.borrow();
    let col = col?;
    let col_id_str = col.identifier().to_string();
    let col_index = model.columns.iter().position(|c| c.title == col_id_str)?;
    let (cell_text, icon_name) = match model.rows.get(row as usize)
        .and_then(|r| r.cells.get(col_index))
    {
        Some(c) => (c.text.clone(), c.icon.clone()), None => return None,
    };
    let mtm = MainThreadMarker::new()?;
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0));
    let tcv = NSTableCellView::initWithFrame(mtm.alloc(), frame);
    let tf = NSTextField::labelWithString(&NSString::from_str(&cell_text), mtm);
    tf.setEditable(false);
    tf.setBordered(false);
    tf.setDrawsBackground(false);
    // Add icon if the cell has one and position the text field to the right of it.
    let text_x: f32 = if let Some(icon_name) = icon_name {
        let img = unsafe {
            objc2_app_kit::NSImage::imageWithSystemSymbolName_accessibilityDescription(
                &NSString::from_str(&icon_name), None,
            )
        };
        if let Some(img) = img {
            let icon_frame = NSRect::new(NSPoint::new(2.0, 2.0), NSSize::new(14.0, 14.0));
            let iv = objc2_app_kit::NSImageView::initWithFrame(mtm.alloc(), icon_frame);
            iv.setImage(Some(&img));
            tcv.addSubview(&iv);
            20.0
        } else { 2.0 }
    } else { 2.0 };
    // FR-011: position the text field to the right of the icon so they don't overlap.
    // NSTableCellView auto-resizes its subviews; set a frame with the correct origin.
    let text_frame = NSRect::new(NSPoint::new(text_x as f64, 0.0), NSSize::new(0.0, 0.0));
    unsafe { tf.setFrame(text_frame); }
    tcv.setTextField(Some(&tf));
    tcv.addSubview(&tf);
    Some(Retained::cast(tcv))
}

declare_class!(
    pub(crate) struct TableDelegate;

    unsafe impl ClassType for TableDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "RnTableDelegate";
    }

    impl DeclaredClass for TableDelegate {
        type Ivars = DelegateIvars;
    }

    unsafe impl NSObjectProtocol for TableDelegate {}
    unsafe impl NSControlTextEditingDelegate for TableDelegate {}

    unsafe impl NSTableViewDelegate for TableDelegate {
        #[method_id(tableView:viewForTableColumn:row:)]
        unsafe fn cell_view(
            &self,
            _tv: &NSTableView,
            col: Option<&NSTableColumn>,
            row: NSInteger,
        ) -> Option<Retained<NSView>> {
            make_cell_view(self, col, row)
        }


        #[method(tableViewSelectionDidChange:)]
        unsafe fn selection_changed(&self, notification: &NSNotification) {
            let ov = match notification.object() { Some(o) => o, None => return };
            let tv: &NSTableView = &*(Retained::as_ptr(&ov) as *const NSTableView);
            let inner = &*self.ivars().inner;
            let handler = match self.ivars().handler_select { Some(h) => h, None => return };
            let model = inner.model.borrow();
            // FR-009: collect all selected row RowIds (supports both single and multi-select).
            let index_set = tv.selectedRowIndexes();
            let count = index_set.count();
            if count == 0 { return; }
            let usize_max = usize::MAX as objc2_foundation::NSUInteger;
            let mut idx = index_set.firstIndex();
            let mut selected_ids: Vec<rn_core::data::RowId> = Vec::new();
            while idx != usize_max {
                if let Some(row_data) = model.rows.get(idx) {
                    selected_ids.push(row_data.id);
                }
                idx = index_set.indexGreaterThanIndex(idx);
            }
            drop(model);
            if !selected_ids.is_empty() {
                // FR-009: write selection back to the model so filter_to and update_node are effective.
                {
                    let mut m = inner.model.borrow_mut();
                    m.selection.selected = selected_ids.clone();
                    m.selection.anchor = selected_ids.first().copied();
                }
                rn_core::dispatch_event(Event {
                    source: self.ivars().source,
                    handler,
                    kind: EventKind::Press,
                    payload: EventPayload::None,
                    stop_bubbling: false,
                });
            }
        }
    }
);

impl TableDelegate {
    fn new(
        mtm: MainThreadMarker,
        inner: *const TableSourcesInner,
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

pub(crate) fn make_table_sources(
    mtm: MainThreadMarker,
    model: TableModel,
    source: NodeId,
    handler_select: Option<HandlerId>,
    handler_activate: Option<HandlerId>,
) -> TableSources {
    let inner = Rc::new(TableSourcesInner { model: Rc::new(RefCell::new(model)) });
    let raw: *const TableSourcesInner = Rc::as_ptr(&inner);
    let data = TableDataSource::new(mtm, raw, source);
    let delegate = TableDelegate::new(mtm, raw, source, handler_select, handler_activate);
    TableSources { data, delegate, inner, activate: None }
}

/// Update the model inside existing TableSources after a data refresh.
#[allow(dead_code)]
pub(crate) fn update_table_model(sources: &TableSources, new_model: TableModel) {
    *sources.inner.model.borrow_mut() = new_model;
}
