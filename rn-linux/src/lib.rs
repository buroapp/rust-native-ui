//! GTK4 backend for Rust Native.
//!
//! The `app` module is host-independent by design: its display-server detection
//! is a pure predicate, so it compiles and tests on any host even where GTK4 is
//! not present. Everything that touches GTK itself is gated on the target OS.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub mod app;

#[cfg(target_os = "linux")]
mod renderer;

#[cfg(target_os = "linux")]
pub use renderer::LinuxRenderer;
