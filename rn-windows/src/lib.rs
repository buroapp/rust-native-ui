//! Win32 backend for Rust Native. Scaffold — compiles and reports an explicit
//! "not implemented" error at run time (v0.1 ships macOS first).

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(target_os = "windows")]
mod renderer;

#[cfg(target_os = "windows")]
pub use renderer::WindowsRenderer;
