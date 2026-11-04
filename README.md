# Rust Native

**Rust Native** is a declarative cross-platform native UI framework for Rust —
the React Native mental model applied to native desktop. You describe the UI
as a pure `ComponentTree` and the framework reconciles consecutive snapshots
into a minimal `Changeset` that drives real OS controls through a thin
per-platform renderer.

**Rust Native** is the product; **`rn`** is the crate.

## Features

- One declarative API, three OS backends. v0.1 ships a complete macOS AppKit
  renderer; Windows (`windows` crate) and Linux (`gtk4`) scaffolds are in
  place so the workspace builds green on all three targets from day one.
- O(n) reconciler with keyed-list reordering and prop-level diffs — a change
  to one label mutates one `stringValue` and nothing else.
- Panic-safe handler dispatch (FR-013), main-thread guard that returns
  `Err(RnError::WrongThread)` instead of panicking (FR-012), and crossbeam-
  channel-based update plumbing for background threads (FR-011).
- ~57 native controls — from `NSButton` and `NSTextField` to `NSScrollView`,
  `NSSegmentedControl`, `NSAlert` and real window sheets. Nothing is painted to
  look like a control; it is the control.
- Headless reconciler tests run without a display server — perfect for CI.
- No platform type leaks into consumer crates: `cargo tree -p your-app` shows
  `objc2` / `windows` / `gtk4` only as transitive deps of the backend crate.

## Quickstart

```rust
use rn::backend::MacosRenderer;
use rn::HandlerId;

fn main() -> Result<(), rn::RnError> {
    rn::thread::MainThreadGuard::init_current();
    let mut renderer = MacosRenderer::new().map_err(rn::RnError::Render)?;

    let close = HandlerId::new();
    rn::on(close, |_| std::process::exit(0));

    let press = HandlerId::new();
    rn::on(press, |_| println!("clicked"));

    let tree = rn::ComponentTree::new(
        rn::window()
            .title("Hello, Rust Native")
            .size(400.0, 200.0)
            .on_close(close)
            .child(
                rn::vstack()
                    .padding(16.0)
                    .gap(8.0)
                    .child(rn::label("Hello, World!"))
                    .child(rn::button("Click me").on_press(press).build())
                    .build(),
            )
            .build(),
    );

    rn::render(&mut renderer, tree)?;
    rn::run(&mut renderer)
}
```

Run it:

```bash
cargo run -p hello-world --features macos
```

## Platform support

| Platform | Backend crate | Status |
|----------|---------------|--------|
| macOS 13+ (Apple Silicon + Intel) | `rn-macos` (AppKit via `objc2`) | ✅ v0.1 complete |
| Windows 10 22H2+ | `rn-windows` (Win32 via `windows`) | 🚧 scaffold — targeted for v0.2 |
| Linux (GTK 4.8+) | `rn-linux` (GTK4) | 🚧 scaffold — targeted for v0.3 |

See [`docs/backend-status.md`](docs/backend-status.md) for the per-widget
support matrix and [`docs/benchmarks.md`](docs/benchmarks.md) for reconciler
performance numbers.

## Repository layout

```
rn-core/      Shared types + traits — backends depend only on this.
rn/           Framework: builder API, reconciler, main-thread guard,
              handler registry, update channel, bench harness.
rn-layout/    taffy-backed flexbox layout engine.
rn-macos/     Real AppKit renderer.
rn-windows/   Scaffold (compile-only).
rn-linux/     Scaffold (compile-only).
examples/
  hello-world/    Minimal window + label + button.
  counter/        Button handler mutates state, label updates — demonstrates
                  `rn::update_sender` + `rn::set_tick` + `rn::rerender`.
  gallery/        Layout primitives side by side.
  showcase/       Broad widget gallery.
  settings-demo/  macOS System Settings-style sidebar + grouped rows.
  mail-demo/      Three-pane mail client.
  storybook/      Every component kind, one page each. Start here.
tests/reconciler/  Headless reconciler test suite.
```

## Documentation

- **[Architecture](docs/ARCHITECTURE.md)** — crate map, render pipeline, layout
  invariants, concurrency model, and where to change what.
  Open [`docs/architecture.html`](docs/architecture.html) for the interactive
  diagram.
- [Backend status matrix](docs/backend-status.md) — per-widget support.
- [Benchmarks](docs/benchmarks.md) — reconciler performance.
- Public API contract: [`specs/001-core-framework/contracts/public-api.md`](specs/001-core-framework/contracts/public-api.md)
- Validation guide: [`specs/001-core-framework/quickstart.md`](specs/001-core-framework/quickstart.md)
- Research decisions: [`specs/001-core-framework/research.md`](specs/001-core-framework/research.md)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

at your option.

Source: <https://github.com/buroapp/rust-native-ui>
