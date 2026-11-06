# Table

Native `NSTableView`-backed sortable, resizable table component.

## Usage

```rust
use rn::{table, vstack, window, TableCell, TableColumn, TableModel, TableRow};

let cols = vec![
    TableColumn::new(0, "Name",  160.0).sortable(),
    TableColumn::new(1, "Score",  80.0).sortable(),
    TableColumn::new(2, "Status",100.0),
];
let rows = vec![
    TableRow::new(0, vec![
        TableCell::text("Alpha"),
        TableCell::text("98"),
        TableCell::text("Active"),
    ]),
];
let model = TableModel::new(cols, rows).expect("valid");

let sel_id = rn::HandlerId::new();
rn::on(sel_id, |_| println!("selected"));

let w = window()
    .child(table(model).on_select(sel_id))
    .build();
```

## Props

| Prop | Builder method | Description |
|------|---------------|-------------|
| `TableModelKey` | `table(model)` | The `TableModel` supplying columns and rows |
| `OnSelect` | `.on_select(handler)` | Fires when a row is selected |
| `OnActivate` | `.on_activate(handler)` | Fires on double-click / Enter |

## TableColumn

| Field | Description |
|-------|-------------|
| `id: ColumnId` | Stable column identity |
| `title: String` | Header text shown in the column header |
| `width: f32` | Default column width (points) |
| `min_width: f32` | Minimum resize floor (default 40 pt) |
| `sortable: bool` | Adds `NSSortDescriptor`; clicking header sorts |
| `alignment: TextAlign` | Left / Center / Right (default Left) |

## TableModel

Constructed via `TableModel::new(columns, rows) -> Result<TableModel, String>`.
Returns `Err` if any row's cell count doesn't match the column count.

## Platform notes

- macOS: `NSTableView` (view-based) with alternating row backgrounds, column resizing.
- Windows/Linux: stub (Phase A scope is macOS only).

## Screenshot

![Table & Tree Demo](img/TableTree.png)
