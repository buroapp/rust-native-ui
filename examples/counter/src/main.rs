#![deny(unused_extern_crates)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(all(target_os = "macos", feature = "macos"))]
fn main() -> Result<(), rn::RnError> {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use rn::backend::MacosRenderer;
    use rn::{HandlerId, UpdateMessage};

    env_logger::init();
    rn::thread::MainThreadGuard::init_current();

    let mut renderer = MacosRenderer::new().map_err(rn::RnError::Render)?;

    // Shared counter state. Main thread reads; the per-tick closure increments
    // whenever an Increment message arrives.
    let count = Arc::new(AtomicU32::new(0));

    enum Msg {
        Increment,
    }

    let press_id = HandlerId::new();
    {
        let tx = rn::update_sender();
        rn::on(press_id, move |e| {
            // Compile-time assertion (T052) — the event the handler sees is
            // rn::event::Event, never a platform type.
            let _: rn::event::Event = e;
            let _ = tx.send(UpdateMessage::new(Msg::Increment));
        });
    }

    let close_id = HandlerId::new();
    rn::on(close_id, |_| std::process::exit(0));

    let view = {
        let count = Arc::clone(&count);
        move || {
            let n = count.load(Ordering::Relaxed);
            rn::ComponentTree::new(
                rn::window()
                    .title("Counter")
                    .size(300.0, 180.0)
                    .on_close(close_id)
                    .child(
                        rn::vstack()
                            .padding(16.0)
                            .gap(8.0)
                            .child(rn::label(format!("Count: {n}")))
                            .child(rn::button("Increment").on_press(press_id).build())
                            .build(),
                    )
                    .build(),
            )
        }
    };

    // Initial render.
    rn::render(&mut renderer, view())?;

    // Install a per-tick closure that drains all pending messages (coalescing
    // multiple Increments into a single re-render per tick, FR-011 / G1) and
    // re-renders through the installed apply bridge.
    {
        let count = Arc::clone(&count);
        let view = view.clone();
        rn::set_tick(move || {
            let batch = rn::drain_updates();
            if batch.is_empty() {
                return;
            }
            let mut inc = 0u32;
            for msg in batch {
                match msg.downcast::<Msg>() {
                    Ok(Msg::Increment) => inc += 1,
                    Err(_) => {}
                }
            }
            if inc > 0 {
                count.fetch_add(inc, Ordering::Relaxed);
                if let Err(e) = rn::rerender(view()) {
                    log::error!("counter: rerender failed: {e}");
                }
            }
        });
    }

    rn::run(&mut renderer)
}

#[cfg(not(all(target_os = "macos", feature = "macos")))]
fn main() {
    env_logger::init();
    eprintln!("counter currently requires --features macos on this host");
    eprintln!("See specs/001-core-framework/tasks.md for backend status.");
}
