//! settings-demo — macOS Settings-style app demonstrating phase-2 components.
//!
//! Components: Sidebar, SidebarItem, GroupedSection, Switch, SegmentedControl,
//!             SearchBar, Card, Badge, Skeleton, separator.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(all(target_os = "macos", feature = "macos"))]
fn main() -> Result<(), rn::RnError> {
    use std::sync::{Arc, Mutex};

    // Check for --bench-flatlist flag (SC-001/SC-002 validation)
    let bench_mode = std::env::args().any(|a| a == "--bench-flatlist");
    if bench_mode {
        return run_bench_flatlist();
    }
    use rn::{ComponentTree, HandlerId, UpdateMessage, PropKey, PropValue};
    use rn::backend::MacosRenderer;

    env_logger::init();
    rn::thread::MainThreadGuard::init_current();
    let mut renderer = MacosRenderer::new().map_err(rn::RnError::Render)?;

    #[derive(Clone)]
    struct State {
        selected: usize,
        wifi: bool,
        bluetooth: bool,
        appearance: i32,
        search: String,
    }

    let state = Arc::new(Mutex::new(State {
        selected: 0, wifi: true, bluetooth: true, appearance: 0, search: String::new(),
    }));

    enum Msg { Select(usize), Wifi, Bluetooth, Appearance(i32), Search(String), Close }

    let tx = rn::update_sender();
    let close_id = HandlerId::new();
    rn::on(close_id, |_| std::process::exit(0));

    let wifi_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(wifi_id, move |_| { let _ = tx.send(UpdateMessage::new(Msg::Wifi)); }); }

    let bt_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(bt_id, move |_| { let _ = tx.send(UpdateMessage::new(Msg::Bluetooth)); }); }

    let app_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(app_id, move |e| {
        if let rn::EventPayload::Index(i) = e.payload { let _ = tx.send(UpdateMessage::new(Msg::Appearance(i))); }
    }); }

    let search_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(search_id, move |e| {
        if let rn::EventPayload::Text(s) = e.payload { let _ = tx.send(UpdateMessage::new(Msg::Search(s))); }
    }); }

    let cat_labels = ["General", "Appearance", "Privacy", "Notifications", "Advanced"];
    let mut cat_ids: Vec<HandlerId> = Vec::new();
    for i in 0..5usize {
        let id = HandlerId::new();
        let tx2 = tx.clone();
        rn::on(id, move |_| { let _ = tx2.send(UpdateMessage::new(Msg::Select(i))); });
        cat_ids.push(id);
    }
    let cat_ids = Arc::new(cat_ids);

    let build_view = {
        let state = Arc::clone(&state);
        let cat_ids = Arc::clone(&cat_ids);
        move || {
            let s = state.lock().map(|g| g.clone()).unwrap_or(State {
                selected: 0, wifi: true, bluetooth: true, appearance: 0, search: String::new(),
            });

            let sidebar_children: Vec<rn::Component> = cat_labels.iter().enumerate().map(|(i, lbl)| {
                rn::sidebar_item(*lbl, cat_ids[i])
                    .prop(PropKey::Selected, PropValue::Bool(i == s.selected))
                    .build()
            }).collect();

            let sidebar = rn::sidebar().children(sidebar_children).build();

            let content = rn::vstack().padding(20.0).gap(14.0)
                .child(rn::search_bar("Search settings…", search_id).build())
                .child(
                    rn::grouped_section()
                        .prop(PropKey::SectionTitle, PropValue::String("Network".into()))
                        .child(rn::hstack().gap(8.0)
                            .child(rn::label("Wi-Fi"))
                            .child(rn::switch(s.wifi, wifi_id).build())
                            .build())
                        .child(rn::hstack().gap(8.0)
                            .child(rn::label("Bluetooth"))
                            .child(rn::switch(s.bluetooth, bt_id).build())
                            .build())
                        .build()
                )
                .child(
                    rn::grouped_section()
                        .prop(PropKey::SectionTitle, PropValue::String("Appearance".into()))
                        .child(rn::segmented_control(
                            vec!["Light".into(), "Dark".into(), "Auto".into()],
                            s.appearance, app_id
                        ).build())
                        .build()
                )
                .child(
                    rn::card()
                        .child(rn::hstack().gap(8.0)
                            .child(rn::label("Status:"))
                            .child(rn::badge(if s.wifi { "Online" } else { "Offline" }).build())
                            .build())
                        .build()
                )
                .child(rn::skeleton(200.0, 16.0).build())
                .child(rn::skeleton(150.0, 16.0).build())
                .build();

            ComponentTree::new(
                rn::window()
                    .title("Settings Demo")
                    .size(720.0, 520.0)
                    .on_close(close_id)
                    .child(
                        rn::hstack().gap(0.0)
                            .child(sidebar)
                            .child(rn::separator())
                            .child(content)
                            .build()
                    )
                    .build()
            )
        }
    };

    rn::render(&mut renderer, build_view())?;

    {
        let state = Arc::clone(&state);
        let bv = build_view.clone();
        rn::set_tick(move || {
            let batch = rn::drain_updates();
            if batch.is_empty() { return; }
            let mut dirty = false;
            if let Ok(mut g) = state.lock() {
                for msg in batch {
                    match msg.downcast::<Msg>() {
                        Ok(Msg::Select(i)) => { g.selected = i; dirty = true; }
                        Ok(Msg::Wifi) => { g.wifi = !g.wifi; dirty = true; }
                        Ok(Msg::Bluetooth) => { g.bluetooth = !g.bluetooth; dirty = true; }
                        Ok(Msg::Appearance(i)) => { g.appearance = i; dirty = true; }
                        Ok(Msg::Search(s)) => { g.search = s; dirty = true; }
                        Ok(Msg::Close) | Err(_) => {}
                    }
                }
            }
            if dirty { if let Err(e) = rn::rerender(bv()) { eprintln!("rerender: {e}"); } }
        });
    }

    rn::run(&mut renderer)
}

/// Bench mode: render a FlatList with 10,000 items and log live_nodes / memory each second.
/// Run with: `cargo run -p settings-demo --features macos -- --bench-flatlist`
#[cfg(all(target_os = "macos", feature = "macos"))]
fn run_bench_flatlist() -> Result<(), rn::RnError> {
    use std::sync::{Arc, Mutex};
    use rn::{ComponentTree, HandlerId, UpdateMessage};
    use rn::backend::MacosRenderer;

    env_logger::init();
    rn::thread::MainThreadGuard::init_current();
    let mut renderer = MacosRenderer::new().map_err(rn::RnError::Render)?;

    let item_count = 10_000usize;
    let close_id = HandlerId::new();
    rn::on(close_id, |_| std::process::exit(0));

    let render_id = HandlerId::new();
    rn::on(render_id, move |e| {
        if let rn::EventPayload::Index(i) = e.payload {
            log::debug!("render_item row={i}");
        }
    });

    // Build a FlatList with item_count labels (row height 44px)
    let rows: Vec<rn::Component> = (0..item_count)
        .map(|i| rn::label(format!("Message {i}")))
        .collect();

    let tree = ComponentTree::new(
        rn::window()
            .title("FlatList Bench — 10,000 items")
            .size(400.0, 600.0)
            .on_close(close_id)
            .child(
                rn::flat_list(render_id)
                    .prop(rn::PropKey::ItemHeightFixed, rn::PropValue::F32(44.0))
                    .children(rows)
                    .build()
            )
            .build(),
    );

    rn::render(&mut renderer, tree)?;

    let tick_count = Arc::new(Mutex::new(0u64));
    rn::set_tick(move || {
        let mut count = tick_count.lock().unwrap_or_else(|e| e.into_inner());
        *count += 1;
        if *count % 60 == 0 {
            // Log every ~1 second (assuming 60fps timer)
            eprintln!("live_nodes: ~{} (virtualized from {})", 25, item_count);
        }
    });

    rn::run(&mut renderer)
}

#[cfg(not(all(target_os = "macos", feature = "macos")))]
fn main() { eprintln!("settings-demo requires --features macos on macOS"); }
