//! settings-demo — macOS Settings-style app demonstrating phase-2 components.
//!
//! Components: Sidebar, SidebarItem, GroupedSection, Switch, SegmentedControl,
//!             SearchBar, Card, Badge, Skeleton, separator.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(all(target_os = "macos", feature = "macos"))]
fn main() -> Result<(), rn::RnError> {
    use std::sync::{Arc, Mutex};
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

#[cfg(not(all(target_os = "macos", feature = "macos")))]
fn main() { eprintln!("settings-demo requires --features macos on macOS"); }
