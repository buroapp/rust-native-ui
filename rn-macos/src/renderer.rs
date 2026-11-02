//! macOS AppKit renderer. Phase 3 stub — enough to compile; real control
//! creation lands in T029–T035.

use rn_core::{Changeset, PlatformRenderer, RenderError};

/// AppKit-backed `PlatformRenderer`.
pub struct MacosRenderer {
    _priv: (),
}

impl MacosRenderer {
    pub fn new() -> Result<Self, RenderError> {
        log::debug!("MacosRenderer::new (scaffold)");
        Ok(Self { _priv: () })
    }
}

impl PlatformRenderer for MacosRenderer {
    fn apply(&mut self, _changeset: Changeset) -> Result<(), RenderError> {
        log::debug!("MacosRenderer::apply (scaffold — no-op)");
        Ok(())
    }

    fn run_event_loop(&mut self) -> Result<(), RenderError> {
        Err(RenderError::OsError(
            "macos backend scaffold: run_event_loop not implemented in Phase 2".to_string(),
        ))
    }
}
