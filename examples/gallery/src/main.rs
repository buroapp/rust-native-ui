//! Gallery — exercises every v1 widget in one app.
//!
//! Layout:
//!
//! ```text
//! ┌────────────────────────────────────────┐
//! │ Rust Native — Widget Gallery           │
//! ├────────────────────────────────────────┤
//! │ Name:  [ text input ................ ] │
//! │                                        │
//! │ Hello, <name or "stranger">!           │
//! │                                        │
//! │ Clicks: 0                              │
//! │ [ -- ]  [ ++ ]  [ Reset ]              │
//! │                                        │
//! │ [ Toggle greeting ]                    │
//! │                                        │
//! │ (greeting is visible / hidden)         │
//! └────────────────────────────────────────┘
//! ```
//!
//! Demonstrates:
//! - window + title
//! - vstack + hstack nesting with padding/gap
//! - label with dynamic text
//! - button with on_press
//! - text_input with on_change (EventPayload::Text)
//! - PropKey::Visible toggled live
//! - cross-thread update channel + per-tick drain + coalesced rerender

#![deny(unused_extern_crates)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(all(target_os = "macos", feature = "macos"))]
fn main() -> Result<(), rn::RnError> {
    use std::sync::{Arc, Mutex};

    use rn::backend::MacosRenderer;
    use rn::{HandlerId, PropKey, PropValue, UpdateMessage};

    env_logger::init();
    rn::thread::MainThreadGuard::init_current();

    let mut renderer = MacosRenderer::new().map_err(rn::RnError::Render)?;

    // Application state.
    #[derive(Clone)]
    struct State {
        name: String,
        clicks: i32,
        greeting_visible: bool,
    }

    let state = Arc::new(Mutex::new(State {
        name: String::new(),
        clicks: 0,
        greeting_visible: true,
    }));

    enum Msg {
        SetName(String),
        Inc,
        Dec,
        Reset,
        ToggleGreeting,
    }

    // Handlers all funnel into Msg via the update channel so state mutation
    // and rerender happen on the main thread's per-tick drain.
    let tx = rn::update_sender();

    let name_change = HandlerId::new();
    {
        let tx = tx.clone();
        rn::on(name_change, move |e| {
            if let rn::EventPayload::Text(s) = e.payload {
                let _ = tx.send(UpdateMessage::new(Msg::SetName(s)));
            }
        });
    }

    let inc = HandlerId::new();
    {
        let tx = tx.clone();
        rn::on(inc, move |_| {
            let _ = tx.send(UpdateMessage::new(Msg::Inc));
        });
    }

    let dec = HandlerId::new();
    {
        let tx = tx.clone();
        rn::on(dec, move |_| {
            let _ = tx.send(UpdateMessage::new(Msg::Dec));
        });
    }

    let reset = HandlerId::new();
    {
        let tx = tx.clone();
        rn::on(reset, move |_| {
            let _ = tx.send(UpdateMessage::new(Msg::Reset));
        });
    }

    let toggle = HandlerId::new();
    {
        let tx = tx.clone();
        rn::on(toggle, move |_| {
            let _ = tx.send(UpdateMessage::new(Msg::ToggleGreeting));
        });
    }

    let close = HandlerId::new();
    rn::on(close, |_| std::process::exit(0));

    // View function — pure value from state.
    let view = {
        let state = Arc::clone(&state);
        move || {
            let s = state.lock().map(|g| g.clone()).unwrap_or(State {
                name: String::new(),
                clicks: 0,
                greeting_visible: true,
            });

            let greeting_text = if s.name.is_empty() {
                "Hello, stranger!".to_string()
            } else {
                format!("Hello, {}!", s.name)
            };

            // Build the greeting label with an explicit Visible prop. The
            // reconciler diffs this on each render; the macOS backend applies
            // it via `setHidden:`.
            let mut greeting_label = rn::Component::new(rn::ComponentKind::Label);
            greeting_label
                .props
                .insert(PropKey::Text, PropValue::String(greeting_text));
            greeting_label
                .props
                .insert(PropKey::Visible, PropValue::Bool(s.greeting_visible));

            rn::ComponentTree::new(
                rn::window()
                    .title("Rust Native — Widget Gallery")
                    .size(420.0, 320.0)
                    .on_close(close)
                    .child(
                        rn::vstack()
                            .padding(20.0)
                            .gap(12.0)
                            // Name row: label + text input side by side.
                            .child(
                                rn::hstack()
                                    .gap(8.0)
                                    .child(rn::label("Name:"))
                                    .child(
                                        rn::text_input()
                                            .placeholder("Type your name…")
                                            .value(s.name.clone())
                                            .on_change(name_change)
                                            .build(),
                                    )
                                    .build(),
                            )
                            .child(greeting_label)
                            // Click counter line.
                            .child(rn::label(format!("Clicks: {}", s.clicks)))
                            // Button row: --, ++, Reset.
                            .child(
                                rn::hstack()
                                    .gap(8.0)
                                    .child(rn::button("−").on_press(dec).build())
                                    .child(rn::button("+").on_press(inc).build())
                                    .child(rn::button("Reset").on_press(reset).build())
                                    .build(),
                            )
                            // Toggle greeting button.
                            .child(rn::button("Toggle greeting").on_press(toggle).build())
                            .build(),
                    )
                    .build(),
            )
        }
    };

    // Initial render.
    rn::render(&mut renderer, view())?;

    // Per-tick drain + rerender when state changes.
    {
        let state = Arc::clone(&state);
        let view = view.clone();
        rn::set_tick(move || {
            let batch = rn::drain_updates();
            if batch.is_empty() {
                return;
            }
            let mut dirty = false;
            if let Ok(mut g) = state.lock() {
                for msg in batch {
                    match msg.downcast::<Msg>() {
                        Ok(Msg::SetName(name)) => {
                            g.name = name;
                            dirty = true;
                        }
                        Ok(Msg::Inc) => {
                            g.clicks += 1;
                            dirty = true;
                        }
                        Ok(Msg::Dec) => {
                            g.clicks -= 1;
                            dirty = true;
                        }
                        Ok(Msg::Reset) => {
                            g.clicks = 0;
                            dirty = true;
                        }
                        Ok(Msg::ToggleGreeting) => {
                            g.greeting_visible = !g.greeting_visible;
                            dirty = true;
                        }
                        Err(_) => {}
                    }
                }
            }
            if dirty {
                if let Err(e) = rn::rerender(view()) {
                    eprintln!("gallery: rerender failed: {e}");
                }
            }
        });
    }

    rn::run(&mut renderer)
}

#[cfg(not(all(target_os = "macos", feature = "macos")))]
fn main() {
    env_logger::init();
    eprintln!("gallery currently requires --features macos on this host");
}
