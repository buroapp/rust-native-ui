//! settings-demo — macOS System Settings-style app.
//!
//! Sidebar: icon + label per category, no explicit separator (NSSplitViewController
//! draws its own divider). Content switches on selection.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(all(target_os = "macos", feature = "macos"))]
fn main() -> Result<(), rn::RnError> {
    use std::sync::{Arc, Mutex};
    use rn::{ComponentTree, HandlerId, PropKey, PropValue, UpdateMessage};
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
    }

    let state = Arc::new(Mutex::new(State {
        selected: 0,
        wifi: true,
        bluetooth: true,
        appearance: 0,
    }));

    enum Msg { Select(usize), Wifi, Bluetooth, Appearance(i32) }

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

    // Each category: (label, SF Symbol icon)
    let categories: &[(&str, &str)] = &[
        ("General",        "gearshape"),
        ("Appearance",     "paintpalette"),
        ("Privacy & Security", "hand.raised"),
        ("Notifications",  "bell.badge"),
        ("Wi-Fi",          "wifi"),
        ("Bluetooth",      "wave.3.right"),
        ("Sound",          "speaker.wave.2"),
        ("Battery",        "battery.100"),
        ("Displays",       "display"),
        ("Keyboard",       "keyboard"),
        ("Trackpad",       "trackpad"),
        ("Mouse",          "computermouse"),
        ("Advanced",       "slider.horizontal.3"),
    ];

    let mut cat_ids: Vec<HandlerId> = Vec::new();
    for i in 0..categories.len() {
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
                selected: 0, wifi: true, bluetooth: true, appearance: 0,
            });

            // ---- Sidebar ----
            let sidebar_items: Vec<rn::Component> = categories
                .iter()
                .enumerate()
                .map(|(i, (lbl, icon))| {
                    rn::sidebar_item(*lbl, cat_ids[i])
                        .icon(*icon)
                        .prop(PropKey::Selected, PropValue::Bool(i == s.selected))
                        .build()
                })
                .collect();

            // Items go directly into sidebar — it provides its own NSScrollView.
            let sidebar = rn::sidebar()
                .prop(PropKey::SidebarWidth, PropValue::F32(220.0))
                .children(sidebar_items)
                .build();

            // ---- Content (switches by selection) ----
            let (page_title, page_icon) = categories[s.selected];
            let page_desc = page_title; // subhead reuses title for now

            let make_row = |title: &str, icon: &str| {
                rn::list_tile(title)
                    .prop(PropKey::IconName, PropValue::String(icon.into()))
                    .prop(PropKey::ShowChevron, PropValue::Bool(true))
                    .build()
            };

            let make_toggle_row = |title: &str, on: bool, id: rn::HandlerId| {
                rn::hstack()
                    .justify_between()
                    .flex(1.0)
                    .child(rn::label(title))
                    .child(rn::switch(on, id).build())
                    .build()
            };

            // Build content specific to the selected page.
            let mut page_content = rn::vstack().padding(20.0).gap(12.0).flex(1.0)
                // Page header
                .child(
                    rn::hstack().gap(10.0)
                        .child(rn::image(page_icon).build())
                        .child(rn::vstack()
                            .child(rn::label(page_title))
                            .child(rn::label(page_desc))
                            .build())
                        .build()
                )
                .child(rn::separator());

            page_content = match s.selected {
                // General
                0 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("About", "info.circle"))
                        .child(rn::divider())
                        .child(make_row("Software Update", "arrow.triangle.2.circlepath"))
                        .child(rn::divider())
                        .child(make_row("Storage", "internaldrive"))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("AirDrop & Continuity", "antenna.radiowaves.left.and.right"))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("Date & Time", "clock"))
                        .child(rn::divider())
                        .child(make_row("Language & Region", "globe"))
                        .child(rn::divider())
                        .child(make_row("Sharing", "square.and.arrow.up"))
                        .build()),
                // Appearance
                1 => page_content
                    .child(rn::grouped_section()
                        .child(
                            rn::vstack().gap(8.0)
                                .child(rn::label("Appearance"))
                                .child(rn::segmented_control(
                                    vec!["Light".into(), "Dark".into(), "Auto".into()],
                                    s.appearance, app_id,
                                ).build())
                                .build()
                        )
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("Accent Color", "paintpalette"))
                        .child(rn::divider())
                        .child(make_row("Highlight Color", "circle.fill"))
                        .build()),
                // Wi-Fi
                4 => page_content
                    .child(rn::grouped_section()
                        .child(make_toggle_row("Wi-Fi", s.wifi, wifi_id))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("My Network", "wifi.circle.fill"))
                        .child(rn::divider())
                        .child(make_row("Other Networks…", "wifi.exclamationmark"))
                        .build()),
                // Bluetooth
                5 => page_content
                    .child(rn::grouped_section()
                        .child(make_toggle_row("Bluetooth", s.bluetooth, bt_id))
                        .build()),
                // Everything else — generic placeholder
                _ => page_content
                    .child(rn::grouped_section()
                        .child(make_row(
                            &format!("{page_title} settings coming soon"),
                            "ellipsis.circle",
                        ))
                        .build()),
            };

            let content = rn::content_area()
                .child(page_content.build())
                .build();

            ComponentTree::new(
                rn::window()
                    .modern(true)
                    .title("Settings")
                    .size(760.0, 560.0)
                    .on_close(close_id)
                    .child(
                        rn::hstack().gap(0.0).flex(1.0)
                            .child(sidebar)
                            // No explicit separator — NSSplitViewController draws its own
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
                        Ok(Msg::Wifi)       => { g.wifi = !g.wifi; dirty = true; }
                        Ok(Msg::Bluetooth)  => { g.bluetooth = !g.bluetooth; dirty = true; }
                        Ok(Msg::Appearance(i)) => { g.appearance = i; dirty = true; }
                        Err(_) => {}
                    }
                }
            }
            if dirty {
                if let Err(e) = rn::rerender(bv()) { eprintln!("rerender: {e}"); }
            }
        });
    }

    rn::run(&mut renderer)
}

#[cfg(not(all(target_os = "macos", feature = "macos")))]
fn main() { eprintln!("settings-demo requires --features macos on macOS"); }
