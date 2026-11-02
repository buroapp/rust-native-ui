use rn_core::{Changeset, PlatformRenderer, RenderError};

pub struct WindowsRenderer;

impl WindowsRenderer {
    pub fn new() -> Result<Self, RenderError> {
        Ok(Self)
    }
}

impl PlatformRenderer for WindowsRenderer {
    fn apply(&mut self, _changeset: Changeset) -> Result<(), RenderError> {
        Ok(())
    }

    fn run_event_loop(&mut self) -> Result<(), RenderError> {
        Err(RenderError::OsError(
            "windows backend not implemented in v0.1".to_string(),
        ))
    }
}
