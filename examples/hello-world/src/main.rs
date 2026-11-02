#![deny(unused_extern_crates)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(all(target_os = "macos", feature = "macos"))]
fn main() -> Result<(), rn::RnError> {
    use rn::backend::MacosRenderer;
    use rn::HandlerId;

    env_logger::init();
    rn::thread::MainThreadGuard::init_current();

    let mut renderer = MacosRenderer::new().map_err(rn::RnError::Render)?;

    let close_id = HandlerId::new();
    rn::on(close_id, |_evt| {
        log::info!("hello-world: window closed, exiting");
        std::process::exit(0);
    });

    let press_id = HandlerId::new();
    rn::on(press_id, |_evt| {
        log::info!("hello-world: button pressed");
    });

    let tree = rn::ComponentTree::new(
        rn::window()
            .title("Hello, Rust Native")
            .size(400.0, 200.0)
            .on_close(close_id)
            .child(
                rn::vstack()
                    .padding(16.0)
                    .gap(8.0)
                    .child(rn::label("Hello, World!"))
                    .child(rn::button("Click me").on_press(press_id).build())
                    .build(),
            )
            .build(),
    );

    rn::render(&mut renderer, tree)?;
    rn::run(&mut renderer)
}

#[cfg(not(all(target_os = "macos", feature = "macos")))]
fn main() {
    env_logger::init();
    eprintln!("hello-world currently requires --features macos on this host");
    eprintln!("See specs/001-core-framework/tasks.md for backend status.");
}
