#[cfg(not(all(target_os = "macos", feature = "macos")))]
fn main() {
    eprintln!("table-demo requires --features macos on macOS");
}

#[cfg(all(target_os = "macos", feature = "macos"))]
fn main() {
    use rn::{command, hstack, label, table, tree, vstack, window};
    use rn::{
        CommandPlacement, CommandSet, KeyBinding, Modifiers,
        TableCell, TableColumn, TableModel, TableRow,
        TreeModel, TreeNode,
    };

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

    // ── Table: 8 people × 5 columns ──────────────────────────────────────────
    let columns = vec![
        TableColumn::new(0, "Name",       160.0).sortable(),
        TableColumn::new(1, "Role",       140.0).sortable(),
        TableColumn::new(2, "Department", 140.0).sortable(),
        TableColumn::new(3, "Location",   120.0),
        TableColumn::new(4, "Status",     100.0),
    ];
    let people: &[(&str, &str, &str, &str, &str)] = &[
        ("Alice Müller",  "Engineer",      "Platform",       "Berlin", "Active"),
        ("Bob Chen",      "Product",       "Growth",         "SF",     "Active"),
        ("Carol Novak",   "Designer",      "Design",         "Remote", "Active"),
        ("David Singh",   "Engineering",   "Infrastructure", "London", "On leave"),
        ("Eve Tanaka",    "Data Scientist","Analytics",      "Tokyo",  "Active"),
        ("Frank Osei",    "Engineer",      "Backend",        "Accra",  "Active"),
        ("Grace Kim",     "PM",            "Payments",       "Seoul",  "Active"),
        ("Héctor López",  "Engineer",      "Frontend",       "Madrid", "Active"),
    ];
    let rows: Vec<TableRow> = people.iter().enumerate().map(|(i, (n, r, d, l, s))| {
        TableRow::new(i as u64, vec![
            TableCell::text(*n), TableCell::text(*r), TableCell::text(*d),
            TableCell::text(*l), TableCell::text(*s),
        ])
    }).collect();
    let model = match TableModel::new(columns, rows) {
        Ok(m) => m, Err(e) => { eprintln!("bad model: {e}"); return; }
    };
    let sel_id = rn::HandlerId::new();
    rn::on(sel_id, |_| println!("table row selected"));

    // ── Tree: file-system-style hierarchy ────────────────────────────────────
    let tree_model = TreeModel::new(vec![
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

    let component_tree = rn::ComponentTree::new(
        window()
            .modern(true)
            .title("Table & Tree Demo")
            .size(1000.0, 560.0)
            .on_close(close_id)
            .commands(set)
            .child(
                hstack()
                    .flex(1.0)
                    .child(
                        vstack()
                            .prop(rn::PropKey::Width, rn::PropValue::F32(220.0))
                            .flex(1.0)
                            .child(label("Files"))
                            .child(tree(tree_model).on_select(tree_sel_id))
                    )
                    .child(
                        vstack()
                            .flex(1.0)
                            .child(label("People"))
                            .child(table(model).on_select(sel_id))
                    )
            )
            .build()
    );

    rn::render(&mut renderer, component_tree).expect("render");
    rn::run(&mut renderer).expect("run");
}
