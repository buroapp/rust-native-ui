#[cfg(not(all(target_os = "macos", feature = "macos")))]
fn main() {
    eprintln!("table-demo requires --features macos on macOS");
}

#[cfg(all(target_os = "macos", feature = "macos"))]
fn main() {
    use rn::{command, table, window, vstack};
    use rn::{
        CommandPlacement, CommandSet, KeyBinding, Modifiers,
        TableCell, TableColumn, TableModel, TableRow,
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

    // TableModel: Name, Role, Department, Location, Status
    let columns = vec![
        TableColumn::new(0, "Name", 160.0).sortable(),
        TableColumn::new(1, "Role", 140.0).sortable(),
        TableColumn::new(2, "Department", 140.0).sortable(),
        TableColumn::new(3, "Location", 120.0),
        TableColumn::new(4, "Status", 100.0),
    ];
    let data: &[(&str, &str, &str, &str, &str)] = &[
        ("Alice Müller",  "Engineer",      "Platform",    "Berlin",    "Active"),
        ("Bob Chen",      "Product",       "Growth",      "SF",        "Active"),
        ("Carol Novak",   "Designer",      "Design",      "Remote",    "Active"),
        ("David Singh",   "Engineering",   "Infrastructure","London",  "On leave"),
        ("Eve Tanaka",    "Data Scientist","Analytics",   "Tokyo",     "Active"),
        ("Frank Osei",    "Engineer",      "Backend",     "Accra",     "Active"),
        ("Grace Kim",     "PM",            "Payments",    "Seoul",     "Active"),
        ("Héctor López",  "Engineer",      "Frontend",    "Madrid",    "Active"),
    ];
    let rows: Vec<TableRow> = data.iter().enumerate().map(|(i, (name, role, dept, loc, st))| {
        TableRow::new(i as u64, vec![
            TableCell::text(*name),
            TableCell::text(*role),
            TableCell::text(*dept),
            TableCell::text(*loc),
            TableCell::text(*st),
        ])
    }).collect();
    let model = match TableModel::new(columns, rows) { Ok(m) => m, Err(e) => { eprintln!("bad model: {e}"); return; } };

    let select_id = rn::HandlerId::new();
    rn::on(select_id, |_| println!("row selected"));

    let tree = rn::ComponentTree::new(
        window()
            .modern(true)
            .title("Table Demo")
            .size(760.0, 520.0)
            .on_close(close_id)
            .commands(set)
            .child(
                vstack()
                    .flex(1.0)
                    .child(table(model).on_select(select_id))
            )
            .build()
    );

    rn::render(&mut renderer, tree).expect("render");
    rn::run(&mut renderer).expect("run");
}
