//! Showcase — every v1 AppKit widget in one window.
//!
//! ```text
//! ┌───────────────────────────────────────────────────┐
//! │ Rust Native — AppKit Showcase                     │
//! ├───────────────────────────────────────────────────┤
//! │ Label        Hello, Rust Native!                  │
//! │ Button       [ Click me ]  Clicked: 0             │
//! │ TextInput    [ _____________ ]                    │
//! │ Checkbox     ☑ Enable dark mode                   │
//! ├───────────────────────────────────────────────────┤
//! │ Slider       ←───────●────────→  42.0             │
//! │ Progress     ████████░░░░░░░░    42%               │
//! │ Stepper      10   [▾▴]                            │
//! ├───────────────────────────────────────────────────┤
//! │ Dropdown     [ Option A           ▾ ]             │
//! │ RadioGroup   ◉ Small  ○ Medium  ○ Large           │
//! ├───────────────────────────────────────────────────┤
//! │ Color well   [■]  R:255 G:128 B:0                 │
//! │ Date picker  [ 2026-11-02        ]                │
//! │ Image        ⭐                                   │
//! ├───────────────────────────────────────────────────┤
//! │ TextArea     [ multi-line                         │
//! │               text area          ]                │
//! │ Separator    ─────────────────────────────────    │
//! │ Status       (live feedback from widgets)         │
//! └───────────────────────────────────────────────────┘
//! ```

#![deny(unused_extern_crates)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(all(target_os = "macos", feature = "macos"))]
fn main() -> Result<(), rn::RnError> {
    use std::sync::{Arc, Mutex};
    use rn::{ColorValue, HandlerId, PropKey, PropValue, UpdateMessage};
    use rn::backend::MacosRenderer;

    env_logger::init();
    rn::thread::MainThreadGuard::init_current();

    let mut renderer = MacosRenderer::new().map_err(rn::RnError::Render)?;

    // Application state
    #[derive(Clone)]
    struct State {
        clicks: i32,
        input_text: String,
        checkbox_on: bool,
        slider_val: f32,
        stepper_val: f32,
        dropdown_idx: i32,
        radio_idx: i32,
        color: ColorValue,
        status: String,
    }

    let state = Arc::new(Mutex::new(State {
        clicks: 0,
        input_text: String::new(),
        checkbox_on: false,
        slider_val: 42.0,
        stepper_val: 10.0,
        dropdown_idx: 0,
        radio_idx: 0,
        color: ColorValue::rgb(255, 128, 0),
        status: "Interact with any widget above".to_string(),
    }));

    enum Msg {
        Click,
        InputChange(String),
        CheckboxToggle,
        SliderChange(f32),
        StepperChange(f32),
        DropdownChange(i32),
        RadioChange(i32),
        ColorChange,
        DateChange,
    }

    let tx = rn::update_sender();

    // Handlers
    let close_id = HandlerId::new();
    rn::on(close_id, |_| std::process::exit(0));

    let click_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(click_id, move |_| { let _ = tx.send(UpdateMessage::new(Msg::Click)); }); }

    let input_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(input_id, move |e| {
        if let rn::EventPayload::Text(s) = e.payload { let _ = tx.send(UpdateMessage::new(Msg::InputChange(s))); }
    }); }

    let checkbox_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(checkbox_id, move |_| { let _ = tx.send(UpdateMessage::new(Msg::CheckboxToggle)); }); }

    let slider_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(slider_id, move |e| {
        if let rn::EventPayload::Float(v) = e.payload { let _ = tx.send(UpdateMessage::new(Msg::SliderChange(v))); }
    }); }

    let stepper_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(stepper_id, move |e| {
        if let rn::EventPayload::Float(v) = e.payload { let _ = tx.send(UpdateMessage::new(Msg::StepperChange(v))); }
    }); }

    let dropdown_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(dropdown_id, move |e| {
        if let rn::EventPayload::Index(i) = e.payload { let _ = tx.send(UpdateMessage::new(Msg::DropdownChange(i))); }
        else { let _ = tx.send(UpdateMessage::new(Msg::DropdownChange(0))); }
    }); }

    let radio_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(radio_id, move |e| {
        if let rn::EventPayload::Index(i) = e.payload { let _ = tx.send(UpdateMessage::new(Msg::RadioChange(i))); }
        else { let _ = tx.send(UpdateMessage::new(Msg::RadioChange(0))); }
    }); }

    let color_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(color_id, move |_| { let _ = tx.send(UpdateMessage::new(Msg::ColorChange)); }); }

    let date_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(date_id, move |_| { let _ = tx.send(UpdateMessage::new(Msg::DateChange)); }); }

    // macos_ui parity demo handlers
    let star_id = HandlerId::new();
    let back_id = HandlerId::new();
    let help_id = HandlerId::new();
    let disc_id = HandlerId::new();
    let gear_id = HandlerId::new();
    let popupb_id = HandlerId::new();
    for id in [star_id, back_id, help_id, disc_id, gear_id, popupb_id] {
        let tx = tx.clone();
        rn::on(id, move |_| { let _ = tx.send(UpdateMessage::new(Msg::DateChange)); });
    }

    let dropdown_options = vec![
        "Option A".to_string(),
        "Option B".to_string(),
        "Option C".to_string(),
    ];
    let radio_options = vec![
        "Small".to_string(),
        "Medium".to_string(),
        "Large".to_string(),
    ];

    // View builder
    let view = {
        let state = Arc::clone(&state);
        let dropdown_options = dropdown_options.clone();
        let radio_options = radio_options.clone();
        move || {
            let s = state.lock().map(|g| g.clone()).unwrap_or(State {
                clicks: 0,
                input_text: String::new(),
                checkbox_on: false,
                slider_val: 0.0,
                stepper_val: 0.0,
                dropdown_idx: 0,
                radio_idx: 0,
                color: ColorValue::rgb(255, 128, 0),
                status: String::new(),
            });

            rn::ComponentTree::new(
                rn::window()
                    .title("Rust Native — AppKit Showcase")
                    .size(480.0, 680.0)
                    .on_close(close_id)
                    .child(
                        rn::vstack()
                            .padding(16.0)
                            .gap(10.0)
                            // ── Basic widgets ──────────────────────────────
                            .child(rn::label("── Basic widgets ─────────────────────────────"))
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::label_w("Label:", 100.0))
                                    .child(rn::label("Hello, Rust Native!"))
                                    .build()
                            )
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::label_w("Button:", 100.0))
                                    .child(rn::button("Click me").on_press(click_id).build())
                                    .child(rn::label(format!("Clicked: {}", s.clicks)))
                                    .build()
                            )
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::label_w("TextInput:", 100.0))
                                    .child(
                                        rn::text_input()
                                            .placeholder("Type here…")
                                            .value(s.input_text.clone())
                                            .on_change(input_id)
                                            .build()
                                    )
                                    .build()
                            )
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::label_w("Checkbox:", 100.0))
                                    .child(
                                        rn::checkbox("Enable dark mode", s.checkbox_on)
                                            .on_change(checkbox_id)
                                            .build()
                                    )
                                    .build()
                            )
                            .child(rn::separator())
                            // ── Range controls ─────────────────────────────
                            .child(rn::label("── Range controls ────────────────────────────"))
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::label_w("Slider:", 100.0))
                                    .child(
                                        rn::slider(s.slider_val, 0.0, 100.0)
                                            .on_change(slider_id)
                                            .build()
                                    )
                                    .child(rn::label(format!("{:.0}", s.slider_val)))
                                    .build()
                            )
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::label_w("Progress:", 100.0))
                                    .child(rn::progress_bar(s.slider_val).build())
                                    .child(rn::label(format!("{:.0}%", s.slider_val)))
                                    .build()
                            )
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::label_w("Stepper:", 100.0))
                                    .child(
                                        rn::stepper(s.stepper_val, 0.0, 100.0, 1.0)
                                            .on_change(stepper_id)
                                            .build()
                                    )
                                    .child(rn::label(format!("{:.0}", s.stepper_val)))
                                    .build()
                            )
                            .child(rn::separator())
                            // ── Selection controls ──────────────────────────
                            .child(rn::label("── Selection controls ────────────────────────"))
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::label_w("Dropdown:", 100.0))
                                    .child(
                                        rn::dropdown(dropdown_options.clone(), s.dropdown_idx)
                                            .on_change(dropdown_id)
                                            .build()
                                    )
                                    .build()
                            )
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::label_w("Radio:", 100.0))
                                    .child(
                                        rn::radio_group(radio_options.clone(), s.radio_idx)
                                            .on_change(radio_id)
                                            .build()
                                    )
                                    .build()
                            )
                            .child(rn::separator())
                            // ── Media & date ────────────────────────────────
                            .child(rn::label("── Media & date ──────────────────────────────"))
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::label_w("Color well:", 100.0))
                                    .child(rn::color_well(s.color).on_change(color_id).build())
                                    .child(rn::label(format!(
                                        "R:{} G:{} B:{}", s.color.r, s.color.g, s.color.b
                                    )))
                                    .build()
                            )
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::label_w("Date picker:", 100.0))
                                    .child(rn::date_picker("2026-11-02").on_change(date_id).build())
                                    .build()
                            )
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::label_w("Image (SF):", 100.0))
                                    .child(rn::image("star.fill").build())
                                    .build()
                            )
                            .child(rn::separator())
                            // ── Multi-line ──────────────────────────────────
                            .child(rn::label("── Multi-line text ───────────────────────────"))
                            .child(
                                rn::text_area("Type multiple\nlines here…")
                                    .build()
                            )
                            .child(rn::separator())
                            // ── macos_ui parity widgets ─────────────────────
                            .child(rn::label("── macos_ui parity ────────────────────────────"))
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::icon_button("star", star_id).build())
                                    .child(rn::back_button(back_id).build())
                                    .child(rn::help_button("Help", help_id).build())
                                    .child(rn::disclosure_button("Disclosure", disc_id).build())
                                    .child(rn::toolbar_icon_button("gear", gear_id).build())
                                    .child(rn::toolbar_divider().build())
                                    .child(rn::popup_button(
                                        vec!["Option A".into(), "Option B".into()],
                                        0, popupb_id,
                                    ).build())
                                    .build()
                            )
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::label_w("Capacity", 70.0))
                                    .child(rn::capacity_indicator(70.0, 100.0).build())
                                    .build()
                            )
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::label_w("Rating", 70.0))
                                    .child(rn::rating_indicator(4.0, 5.0).build())
                                    .child(rn::label_w("Relevance", 70.0))
                                    .child(rn::relevance_indicator(0.75).build())
                                    .build()
                            )
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::label_w("Time", 70.0))
                                    .child(rn::time_picker("10:30").build())
                                    .build()
                            )
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::list_tile("ListTile row")
                                        .prop(PropKey::IconName, PropValue::String("folder".into()))
                                        .build())
                                    .build()
                            )
                            .child(rn::tooltip("This is a tooltip").build())
                            .child(rn::separator())
                            // ── Design system (Phase 2) ──────────────────────
                            .child(rn::label("── Design System ─────────────────────────────"))
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::card()
                                        .child(rn::label("Card component"))
                                        .build())
                                    .build()
                            )
                            .child(
                                rn::hstack().gap(8.0)
                                    .child(rn::badge("Default").build())
                                    .child(rn::badge("Destructive").build())
                                    .child(rn::badge("Success").build())
                                    .build()
                            )
                            .child(rn::skeleton(180.0, 16.0).build())
                            .child(rn::skeleton(140.0, 16.0).build())
                            .child(rn::separator())
                            // ── Status bar ──────────────────────────────────
                            .child(rn::label(format!("Status: {}", s.status)))
                            .build(),
                    )
                    .build(),
            )
        }
    };

    rn::render(&mut renderer, view())?;

    {
        let state = Arc::clone(&state);
        let view = view.clone();
        rn::set_tick(move || {
            let batch = rn::drain_updates();
            if batch.is_empty() { return; }
            let mut dirty = false;
            if let Ok(mut g) = state.lock() {
                for msg in batch {
                    match msg.downcast::<Msg>() {
                        Ok(Msg::Click) => {
                            g.clicks += 1;
                            g.status = format!("Button clicked {} time(s)", g.clicks);
                            dirty = true;
                        }
                        Ok(Msg::InputChange(s)) => {
                            g.status = format!("Text input: \"{s}\"");
                            g.input_text = s;
                            dirty = true;
                        }
                        Ok(Msg::CheckboxToggle) => {
                            g.checkbox_on = !g.checkbox_on;
                            g.status = format!("Checkbox: {}", if g.checkbox_on { "ON" } else { "OFF" });
                            dirty = true;
                        }
                        Ok(Msg::SliderChange(v)) => {
                            g.slider_val = v;
                            g.status = format!("Slider: {v:.1}");
                            dirty = true;
                        }
                        Ok(Msg::StepperChange(v)) => {
                            g.stepper_val = v;
                            g.status = format!("Stepper: {v:.0}");
                            dirty = true;
                        }
                        Ok(Msg::DropdownChange(i)) => {
                            g.dropdown_idx = i;
                            g.status = format!("Dropdown: option {i}");
                            dirty = true;
                        }
                        Ok(Msg::RadioChange(i)) => {
                            g.radio_idx = i;
                            g.status = format!("Radio: option {i}");
                            dirty = true;
                        }
                        Ok(Msg::ColorChange) => {
                            g.status = "Color well changed".to_string();
                            dirty = true;
                        }
                        Ok(Msg::DateChange) => {
                            g.status = "Date picker changed".to_string();
                            dirty = true;
                        }
                        Err(_) => {}
                    }
                }
            }
            if dirty {
                if let Err(e) = rn::rerender(view()) {
                    eprintln!("showcase: rerender failed: {e}");
                }
            }
        });
    }

    rn::run(&mut renderer)
}

#[cfg(not(all(target_os = "macos", feature = "macos")))]
fn main() {
    eprintln!("showcase requires --features macos on macOS");
}
