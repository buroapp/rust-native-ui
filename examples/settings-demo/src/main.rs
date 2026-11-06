//! settings-demo — macOS System Settings-style app, pixel-level pass.
//!
//! Sidebar: pinned search field, then an Apple Account card and a Family
//! card (native avatar tiles + two-line text), then a flat, continuous,
//! unlabeled item list — the way the real Settings sidebar is actually
//! laid out (no group headers). Content pane: back/forward nav pill,
//! large colored header tile, grouped cards whose rows carry small
//! colored icon tiles, exactly like About/Software Update/Storage do in
//! the real app.

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
        allow_notifications: bool,
        do_not_disturb: bool,
        require_password: bool,
        low_power_mode: bool,
    }

    let state = Arc::new(Mutex::new(State {
        selected: 0,
        wifi: true,
        bluetooth: true,
        appearance: 0,
        allow_notifications: true,
        do_not_disturb: false,
        require_password: true,
        low_power_mode: false,
    }));

    enum Msg {
        Select(usize), Wifi, Bluetooth, Appearance(i32),
        AllowNotifications, DoNotDisturb, RequirePassword, LowPowerMode,
    }

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

    // Visual-only in this demo: no navigation history yet, so these don't
    // move the selection, but the real pill — a genuine NSSegmentedControl
    // wrapped as rn::back_forward_control() — sits in the content header.
    let nav_id = HandlerId::new();
    rn::on(nav_id, |_| {});

    let search_id = HandlerId::new();
    rn::on(search_id, |_| {});

    let account_id = HandlerId::new();
    rn::on(account_id, |_| {});

    let family_id = HandlerId::new();
    rn::on(family_id, |_| {});

    let notif_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(notif_id, move |_| { let _ = tx.send(UpdateMessage::new(Msg::AllowNotifications)); }); }

    let dnd_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(dnd_id, move |_| { let _ = tx.send(UpdateMessage::new(Msg::DoNotDisturb)); }); }

    let reqpw_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(reqpw_id, move |_| { let _ = tx.send(UpdateMessage::new(Msg::RequirePassword)); }); }

    let lpm_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(lpm_id, move |_| { let _ = tx.send(UpdateMessage::new(Msg::LowPowerMode)); }); }

    // (label, SF Symbol, tile tint) — names, order, and colors mirror the
    // real Settings sidebar list.
    let categories: &[(&str, &str, &str)] = &[
        ("General",             "gearshape",              "gray"),
        ("Wi-Fi",               "wifi",                   "blue"),
        ("Bluetooth",           "wave.3.right",           "blue"),
        ("Network",             "globe",                  "blue"),
        ("Notifications",       "bell.badge",             "red"),
        ("Sound",               "speaker.wave.2",         "pink"),
        ("Focus",               "moon.fill",              "indigo"),
        ("Screen Time",         "hourglass",              "blue"),
        ("Lock Screen",         "lock",                    "gray"),
        ("Appearance",          "paintpalette",           "teal"),
        ("Accessibility",       "figure.wave",            "blue"),
        ("Control Center",      "switch.2",               "gray"),
        ("Siri & Spotlight",    "mic",                    "purple"),
        ("Privacy & Security",  "lock.shield",             "blue"),
        ("Desktop & Dock",      "menubar.dock.rectangle", "blue"),
        ("Displays",            "display",                "gray"),
        ("Wallpaper",           "photo",                   "teal"),
        ("Battery",             "battery.100",             "green"),
        ("Keyboard",            "keyboard",                "gray"),
        ("Mouse",               "computermouse",           "gray"),
        ("Trackpad",            "trackpad",                "gray"),
        ("Printers & Scanners", "printer",                 "gray"),
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
                allow_notifications: true, do_not_disturb: false,
                require_password: true, low_power_mode: false,
            });

            // ---- Sidebar ----
            // Pinned header: search, then Apple Account + Family cards
            // (native ListTile rows with a round avatar tile, not a
            // sidebar row — these don't scroll with the item list).
            // Below that: one continuous flat list, no group labels —
            // that is how System Settings actually lays its sidebar out.
            let account_row = rn::list_tile("Evgenii Utkin")
                .prop(PropKey::Subtitle, PropValue::String("Apple Account".into()))
                .prop(PropKey::IconName, PropValue::String("person.crop.circle.fill".into()))
                .icon_color("gray")
                .icon_shape("circle")
                .prop(PropKey::ShowChevron, PropValue::Bool(true))
                .prop(PropKey::OnPress, PropValue::HandlerRef(account_id))
                .build();

            let family_row = rn::list_tile("Family")
                .prop(PropKey::IconName, PropValue::String("person.2.circle.fill".into()))
                .icon_color("orange")
                .icon_shape("circle")
                .prop(PropKey::ShowChevron, PropValue::Bool(true))
                .prop(PropKey::OnPress, PropValue::HandlerRef(family_id))
                .build();

            let sidebar_items: Vec<rn::Component> = categories
                .iter()
                .enumerate()
                .map(|(i, (lbl, icon, color))| {
                    rn::sidebar_item(*lbl, cat_ids[i])
                        .icon(*icon)
                        .icon_color(*color)
                        .prop(PropKey::Selected, PropValue::Bool(i == s.selected))
                        .build()
                })
                .collect();

            let sidebar = rn::sidebar()
                .prop(PropKey::SidebarWidth, PropValue::F32(230.0))
                .child(rn::search_bar("Search", search_id).build())
                .child(account_row)
                .child(family_row)
                .children(sidebar_items)
                .build();

            // ---- Content ----
            let (page_title, page_icon, page_color) = categories[s.selected];

            // Content-row icons use the same small colored tile treatment
            // the real app uses for About/Software Update/Storage/etc.
            let make_row = |title: &str, icon: &str, color: &str| {
                rn::list_tile(title)
                    .prop(PropKey::IconName, PropValue::String(icon.into()))
                    .icon_color(color)
                    .prop(PropKey::ShowChevron, PropValue::Bool(true))
                    .build()
            };

            let make_toggle_row = |title: &str, on: bool, id: rn::HandlerId| {
                rn::hstack()
                    .justify_between()
                    .flex(1.0)
                    .child(rn::label_builder(title).flex(1.0).build())
                    .child(rn::switch(on, id).build())
                    .build()
            };

            let mut page_content = rn::vstack().padding(20.0).gap(12.0).flex(1.0)
                // Back/forward pill, top-left — a real NSSegmentedControl.
                .child(rn::back_forward_control(nav_id).build())
                .child(
                    rn::hstack().gap(10.0).align_center()
                        .child(rn::image(page_icon).icon_color(page_color).build())
                        .child(rn::vstack().flex(1.0)
                            .child(rn::label_builder(page_title).flex(1.0).build())
                            .build())
                        .build()
                )
                .child(rn::separator());

            page_content = match s.selected {
                0 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("About", "info.circle", "gray"))
                        .child(rn::divider())
                        .child(make_row("Software Update", "arrow.triangle.2.circlepath", "gray"))
                        .child(rn::divider())
                        .child(make_row("Storage", "internaldrive", "gray"))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("AirDrop & Continuity", "antenna.radiowaves.left.and.right", "blue"))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("Date & Time", "clock", "blue"))
                        .child(rn::divider())
                        .child(make_row("Language & Region", "globe", "blue"))
                        .child(rn::divider())
                        .child(make_row("Sharing", "square.and.arrow.up", "blue"))
                        .build()),
                9 => page_content
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
                        .child(make_row("Accent Color", "paintpalette", "teal"))
                        .child(rn::divider())
                        .child(make_row("Highlight Color", "circle.fill", "pink"))
                        .build()),
                1 => page_content
                    .child(rn::grouped_section()
                        .child(make_toggle_row("Wi-Fi", s.wifi, wifi_id))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("My Network", "wifi.circle.fill", "blue"))
                        .child(rn::divider())
                        .child(make_row("Other Networks...", "wifi.exclamationmark", "gray"))
                        .build()),
                2 => page_content
                    .child(rn::grouped_section()
                        .child(make_toggle_row("Bluetooth", s.bluetooth, bt_id))
                        .build()),
                4 => page_content
                    .child(rn::grouped_section()
                        .child(make_toggle_row("Allow Notifications", s.allow_notifications, notif_id))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("Messages", "message.fill", "green"))
                        .child(rn::divider())
                        .child(make_row("Mail", "envelope.fill", "blue"))
                        .child(rn::divider())
                        .child(make_row("Calendar", "calendar", "red"))
                        .child(rn::divider())
                        .child(make_row("Safari", "safari", "blue"))
                        .build()),
                5 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("Sound Effects", "speaker.wave.3", "pink"))
                        .child(rn::divider())
                        .child(make_row("Alert Volume", "slider.horizontal.3", "pink"))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("Output Device", "hifispeaker.fill", "gray"))
                        .child(rn::divider())
                        .child(make_row("Input Device", "mic.fill", "gray"))
                        .build()),
                6 => page_content
                    .child(rn::grouped_section()
                        .child(make_toggle_row("Do Not Disturb", s.do_not_disturb, dnd_id))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("Personal", "person.fill", "blue"))
                        .child(rn::divider())
                        .child(make_row("Work", "briefcase.fill", "orange"))
                        .child(rn::divider())
                        .child(make_row("Sleep", "bed.double.fill", "indigo"))
                        .build()),
                8 => page_content
                    .child(rn::grouped_section()
                        .child(make_toggle_row("Require Password", s.require_password, reqpw_id))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("Auto-Lock", "lock.rotation", "gray"))
                        .child(rn::divider())
                        .child(make_row("Screen Saver", "sparkles", "indigo"))
                        .build()),
                13 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("Location Services", "location.fill", "blue"))
                        .child(rn::divider())
                        .child(make_row("Contacts", "person.crop.circle", "blue"))
                        .child(rn::divider())
                        .child(make_row("Camera", "camera.fill", "green"))
                        .child(rn::divider())
                        .child(make_row("Microphone", "mic.fill", "pink"))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("FileVault", "lock.doc.fill", "blue"))
                        .child(rn::divider())
                        .child(make_row("Firewall", "shield.lefthalf.filled", "blue"))
                        .build()),
                17 => page_content
                    .child(rn::grouped_section()
                        .child(make_toggle_row("Low Power Mode", s.low_power_mode, lpm_id))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("Battery Health", "heart.text.square.fill", "green"))
                        .child(rn::divider())
                        .child(make_row("Battery Percentage", "batteryblock.fill", "green"))
                        .build()),
                3 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("Wi-Fi", "wifi", "blue"))
                        .child(rn::divider())
                        .child(make_row("Ethernet", "cable.connector", "gray"))
                        .child(rn::divider())
                        .child(make_row("VPN", "lock.shield", "blue"))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("Firewall", "flame.fill", "orange"))
                        .child(rn::divider())
                        .child(make_row("DNS", "network", "gray"))
                        .build()),
                7 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("App Limits", "hourglass", "blue"))
                        .child(rn::divider())
                        .child(make_row("Downtime", "moon.zzz.fill", "indigo"))
                        .child(rn::divider())
                        .child(make_row("Always Allowed", "checkmark.circle", "green"))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("Content & Privacy", "exclamationmark.shield", "red"))
                        .build()),
                10 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("VoiceOver", "eye.fill", "blue"))
                        .child(rn::divider())
                        .child(make_row("Zoom", "plus.magnifyingglass", "blue"))
                        .child(rn::divider())
                        .child(make_row("Display", "sun.max.fill", "orange"))
                        .child(rn::divider())
                        .child(make_row("Spoken Content", "waveform", "purple"))
                        .build()),
                11 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("Sound", "speaker.wave.2", "pink"))
                        .child(rn::divider())
                        .child(make_row("Wi-Fi", "wifi", "blue"))
                        .child(rn::divider())
                        .child(make_row("Bluetooth", "wave.3.right", "blue"))
                        .child(rn::divider())
                        .child(make_row("Now Playing", "play.circle.fill", "red"))
                        .build()),
                12 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("Siri Voice", "waveform.circle", "purple"))
                        .child(rn::divider())
                        .child(make_row("Spotlight", "magnifyingglass.circle.fill", "gray"))
                        .child(rn::divider())
                        .child(make_row("Announce Notifications", "bell.and.waves.left.and.right", "purple"))
                        .build()),
                14 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("Dock Size", "dock.rectangle", "blue"))
                        .child(rn::divider())
                        .child(make_row("Magnification", "arrow.up.left.and.arrow.down.right", "blue"))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("Mission Control", "square.grid.3x3.fill", "gray"))
                        .child(rn::divider())
                        .child(make_row("Hot Corners...", "rectangle.3.group", "gray"))
                        .build()),
                15 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("Resolution", "rectangle.on.rectangle", "gray"))
                        .child(rn::divider())
                        .child(make_row("Brightness", "sun.max.fill", "orange"))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("Night Shift", "moon.stars.fill", "indigo"))
                        .child(rn::divider())
                        .child(make_row("True Tone", "sparkles", "teal"))
                        .build()),
                16 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("Desktop Picture", "photo.on.rectangle", "teal"))
                        .child(rn::divider())
                        .child(make_row("Dynamic Wallpaper", "photo.stack", "teal"))
                        .child(rn::divider())
                        .child(make_row("Screen Saver", "sparkles", "indigo"))
                        .build()),
                18 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("Keyboard Shortcuts...", "command", "gray"))
                        .child(rn::divider())
                        .child(make_row("Text Replacements", "textformat.abc", "blue"))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("Dictation", "mic.fill", "pink"))
                        .child(rn::divider())
                        .child(make_row("Input Sources", "globe", "blue"))
                        .build()),
                19 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("Tracking Speed", "cursorarrow.motionlines", "gray"))
                        .child(rn::divider())
                        .child(make_row("Scroll Direction", "arrow.up.arrow.down", "gray"))
                        .child(rn::divider())
                        .child(make_row("Secondary Click", "cursorarrow.click.2", "gray"))
                        .build()),
                20 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("Point & Click", "hand.point.up.left.fill", "gray"))
                        .child(rn::divider())
                        .child(make_row("Scroll & Zoom", "arrow.up.left.and.down.right.magnifyingglass", "gray"))
                        .child(rn::divider())
                        .child(make_row("More Gestures", "hand.draw.fill", "gray"))
                        .build()),
                21 => page_content
                    .child(rn::grouped_section()
                        .child(make_row("No printers or scanners found", "printer", "gray"))
                        .build())
                    .child(rn::grouped_section()
                        .child(make_row("Add Printer or Scanner...", "plus.circle", "blue"))
                        .build()),
                _ => page_content
                    .child(rn::grouped_section()
                        .child(make_row(
                            &format!("{page_title} settings coming soon"),
                            "ellipsis.circle",
                            "gray",
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
                    .size(780.0, 640.0)
                    .on_close(close_id)
                    .child(
                        rn::hstack().gap(0.0).flex(1.0)
                            .child(sidebar)
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
                        Ok(Msg::AllowNotifications) => { g.allow_notifications = !g.allow_notifications; dirty = true; }
                        Ok(Msg::DoNotDisturb) => { g.do_not_disturb = !g.do_not_disturb; dirty = true; }
                        Ok(Msg::RequirePassword) => { g.require_password = !g.require_password; dirty = true; }
                        Ok(Msg::LowPowerMode) => { g.low_power_mode = !g.low_power_mode; dirty = true; }
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
