#[cfg(not(all(target_os = "macos", feature = "macos")))]
fn main() {
    eprintln!("table-demo requires --features macos on macOS");
}

#[cfg(all(target_os = "macos", feature = "macos"))]
fn main() {
    use rn::{command, label, window, PropKey, PropValue};
    use rn::{AvailabilityRule, CommandPlacement, CommandSet, KeyBinding, Modifiers};

    let mut renderer = rn::backend::MacosRenderer::new()
        .expect("MacosRenderer");

    let new_id    = rn::HandlerId::new();
    let open_id   = rn::HandlerId::new();
    let close_id  = rn::HandlerId::new();

    rn::on(new_id,   |_| println!("Command: New"));
    rn::on(open_id,  |_| println!("Command: Open"));
    rn::on(close_id, |_| std::process::exit(0));

    let mut set = CommandSet::new();
    let mut new_cmd = command(1, "New",  new_id);
    new_cmd.shortcut = Some(KeyBinding { key: "n".to_owned(), modifiers: Modifiers::COMMAND });
    new_cmd.placement = CommandPlacement { menu: "File".to_owned(), order: 10 };
    set.push(new_cmd);

    let mut open_cmd = command(2, "Open…", open_id);
    open_cmd.shortcut = Some(KeyBinding { key: "o".to_owned(), modifiers: Modifiers::COMMAND });
    open_cmd.placement = CommandPlacement { menu: "File".to_owned(), order: 20 };
    set.push(open_cmd);

    let mut unavail = command(3, "Export (unavailable)", rn::HandlerId::new());
    unavail.available = AvailabilityRule::Never;
    unavail.placement = CommandPlacement { menu: "File".to_owned(), order: 30 };
    set.push(unavail);

    let tree = rn::ComponentTree::new(
        window()
            .modern(true)
            .title("Table Demo")
            .size(720.0, 520.0)
            .on_close(close_id)
            .commands(set)
            .child(label("Commands live in the File menu. ⌘N and ⌘O print to stdout. Export is disabled."))
            .build()
    );

    rn::render(&mut renderer, tree).expect("render");
    rn::run(&mut renderer).expect("run");
}
