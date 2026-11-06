use super::{ColumnId, RowId};

/// Text alignment in a table cell or column header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign { #[default] Left, Center, Right }

/// One column declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct TableColumn {
    pub id: ColumnId,
    pub title: String,
    pub width: f32,
    pub min_width: f32,
    pub alignment: TextAlign,
    pub sortable: bool,
}

impl TableColumn {
    pub fn new(id: u64, title: impl Into<String>, width: f32) -> Self {
        TableColumn { id: ColumnId(id), title: title.into(), width, min_width: 40.0,
            alignment: TextAlign::Left, sortable: false }
    }
    pub fn sortable(mut self) -> Self { self.sortable = true; self }
    pub fn min_width(mut self, v: f32) -> Self { self.min_width = v; self }
}

/// One cell in a row.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TableCell {
    pub text: String,
    pub icon: Option<String>,
}

impl TableCell {
    pub fn text(text: impl Into<String>) -> Self { TableCell { text: text.into(), icon: None } }
    pub fn icon(text: impl Into<String>, icon: impl Into<String>) -> Self {
        TableCell { text: text.into(), icon: Some(icon.into()) }
    }
}

/// One row of data.
#[derive(Debug, Clone, PartialEq)]
pub struct TableRow {
    pub id: RowId,
    pub cells: Vec<TableCell>,
    pub selectable: bool,
}

impl TableRow {
    pub fn new(id: u64, cells: Vec<TableCell>) -> Self {
        TableRow { id: RowId(id), cells, selectable: true }
    }
}

/// Sort direction for a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection { Ascending, Descending }

impl SortDirection {
    pub fn flip(self) -> Self {
        match self { Self::Ascending => Self::Descending, Self::Descending => Self::Ascending }
    }
}

/// The column currently sorted and its direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortDescriptor { pub column: ColumnId, pub direction: SortDirection }

/// Current selection in a table.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Selection {
    pub selected: Vec<RowId>,
    pub anchor: Option<RowId>,
}

impl Selection {
    pub fn single(id: RowId) -> Self { Selection { selected: vec![id], anchor: Some(id) } }
    pub fn is_selected(&self, id: RowId) -> bool { self.selected.contains(&id) }
    /// Remove ids that are no longer in the model.
    pub fn filter_to(&mut self, valid: &[RowId]) {
        self.selected.retain(|id| valid.contains(id));
        if let Some(a) = self.anchor { if !valid.contains(&a) { self.anchor = None; } }
    }
}

/// The full table data model.
#[derive(Debug, Clone, PartialEq)]
pub struct TableModel {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<TableRow>,
    pub sort: Option<SortDescriptor>,
    pub selection: Selection,
}

impl TableModel {
    /// Construct a table model. Returns Err if any row has the wrong cell count.
    pub fn new(columns: Vec<TableColumn>, rows: Vec<TableRow>) -> Result<Self, String> {
        for r in &rows {
            if r.cells.len() != columns.len() {
                return Err(format!("Row {:?} has {} cells but {} columns",
                    r.id, r.cells.len(), columns.len()));
            }
        }
        Ok(TableModel { columns, rows, sort: None, selection: Selection::default() })
    }
    pub fn row_ids(&self) -> Vec<RowId> { self.rows.iter().map(|r| r.id).collect() }
    pub fn text_of(&self, row: usize, col: usize) -> &str {
        self.rows.get(row).and_then(|r| r.cells.get(col)).map_or("", |c| &c.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn cell_count_mismatch_is_error() {
        let cols = vec![TableColumn::new(0, "A", 80.0), TableColumn::new(1, "B", 80.0)];
        let bad_row = TableRow::new(0, vec![TableCell::text("only one")]);
        assert!(TableModel::new(cols, vec![bad_row]).is_err());
    }
    #[test] fn valid_model_constructs() {
        let cols = vec![TableColumn::new(0, "A", 80.0)];
        let row = TableRow::new(0, vec![TableCell::text("hello")]);
        let r = TableModel::new(cols, vec![row]);
        assert!(r.is_ok());
        if let Ok(m) = r { assert_eq!(m.text_of(0, 0), "hello"); }
    }
    #[test] fn selection_filters_vanished_row() {
        let mut sel = Selection { selected: vec![RowId(1), RowId(2)], anchor: Some(RowId(2)) };
        sel.filter_to(&[RowId(1)]);
        assert_eq!(sel.selected, vec![RowId(1)]);
        assert_eq!(sel.anchor, None);
    }
}
