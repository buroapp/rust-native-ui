//! macOS AppKit backend for Rust Native. Currently a scaffold — real
//! AppKit integration lands in Phase 3 (US1) of the implementation plan.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(target_os = "macos")]
mod renderer;

#[cfg(target_os = "macos")]
pub use renderer::MacosRenderer;

#[cfg(target_os = "macos")]
pub(crate) mod commands;
#[cfg(target_os = "macos")]
pub(crate) mod table;
#[cfg(target_os = "macos")]
pub(crate) mod tree;

#[cfg(target_os = "macos")]
pub use commands::set_command_flag;
