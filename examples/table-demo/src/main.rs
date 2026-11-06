#[cfg(not(all(target_os = "macos", feature = "macos")))]
fn main() {
    eprintln!("table-demo requires --features macos on macOS");
}

#[cfg(all(target_os = "macos", feature = "macos"))]
fn main() {
    use rn::{
        command, hstack, label, table, tree, vstack, window,
        button,
        CommandPlacement, CommandSet, KeyBinding, Modifiers,
        TableCell, TableColumn, TableModel, TableRow,
        TreeModel, TreeNode,
    };

    // Parse --rows N from CLI args (default 1000).
    let row_count: usize = std::env::args()
        .skip_while(|a| a != "--rows")
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);

    let mut renderer = rn::backend::MacosRenderer::new()
        .expect("MacosRenderer");

    // Commands
    let close_id = rn::HandlerId::new();
    rn::on(close_id, |_| std::process::exit(0));
    let mut set = CommandSet::new();
    let mut close_cmd = command(1, "Close Window", close_id);
    close_cmd.shortcut = Some(KeyBinding { key: "w".to_owned(), modifiers: Modifiers::COMMAND });
    close_cmd.placement = CommandPlacement { menu: "Window".to_owned(), order: 10 };
    set.push(close_cmd);

    // ── Table ────────────────────────────────────────────────────────────────
    let columns = vec![
        TableColumn::new(0, "Name",       160.0).sortable(),
        TableColumn::new(1, "Role",       130.0).sortable(),
        TableColumn::new(2, "Department", 130.0).sortable(),
        TableColumn::new(3, "Location",   110.0),
        TableColumn::new(4, "Status",     100.0),
    ];
    let roles    = ["Engineer", "PM", "Designer", "Data Scientist", "Analyst"];
    let depts    = ["Platform", "Growth", "Design", "Analytics", "Infrastructure"];
    let locs     = ["Berlin", "SF", "Remote", "London", "Tokyo"];
    let statuses = ["Active", "On leave", "Active", "Active", "Draft"];
    let rows: Vec<TableRow> = (0..row_count).map(|i| {
        let n = i % roles.len();
        TableRow::new(i as u64, vec![
            TableCell::text(format!("Person {i}")),
            TableCell::text(roles[n]),
            TableCell::text(depts[n]),
            TableCell::text(locs[n % locs.len()]),
            TableCell::text(statuses[n % statuses.len()]),
        ])
    }).collect();

    let model = match TableModel::new(columns, rows) {
        Ok(m) => m,
        Err(e) => { eprintln!("bad model: {e}"); return; }
    };

    let status_label_id = rn::HandlerId::new();
    let sel_id = rn::HandlerId::new();
    rn::on(sel_id, |_| println!("row selected"));

    // ── Tree ─────────────────────────────────────────────────────────────────
    let make_tree = || TreeModel::new(vec![
        TreeNode::branch(100, "src", vec![
            TreeNode::leaf(101, "main.rs").icon("doc.plaintext"),
            TreeNode::leaf(102, "lib.rs").icon("doc.plaintext"),
            TreeNode::branch(103, "components", vec![
                TreeNode::leaf(104, "button.rs"),
                TreeNode::leaf(105, "table.rs"),
                TreeNode::leaf(106, "tree.rs"),
            ]).expanded(),
        ]).expanded(),
        TreeNode::branch(107, "tests", vec![
            TreeNode::leaf(108, "integration.rs"),
        ]),
        TreeNode::leaf(109, "Cargo.toml").icon("doc.text"),
    ]);

    let tree_sel_id = rn::HandlerId::new();
    rn::on(tree_sel_id, |_| println!("tree node selected"));

    let refresh_id = rn::HandlerId::new();
    rn::on(refresh_id, |_| println!("refresh triggered"));

    let component_tree = rn::ComponentTree::new(
        window()
            .modern(true)
            .title(format!("Table & Tree Demo ({row_count} rows)"))
            .size(1020.0, 580.0)
            .on_close(close_id)
            .commands(set)
            .child(
                hstack()
                    .flex(1.0)
                    .child(
                        vstack()
                            .prop(rn::PropKey::Width, rn::PropValue::F32(230.0))
                            .flex(1.0)
                            .child(
                                hstack()
                                    .child(label("Files"))
                                    .child(button("↺").on_press(refresh_id))
                            )
                            .child(tree(make_tree()).on_select(tree_sel_id))
                    )
                    .child(
                        vstack()
                            .flex(1.0)
                            .child(label(format!("People ({row_count} rows)")))
                            .child(table(model).on_select(sel_id))
                    )
            )
            .build()
    );

    rn::render(&mut renderer, component_tree).expect("render");
    rn::run(&mut renderer).expect("run");
}
