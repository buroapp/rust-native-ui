//! Linux application host and display-server detection.
//!
//! The detection logic is a pure function over the environment so it can be
//! unit-tested on any host, including the macOS and Windows CI legs. Only the
//! GTK application startup that consumes it is platform-gated.

/// Which display protocol the current session is using.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServer {
    /// Wayland compositor.
    Wayland,
    /// X11 server.
    X11,
    /// Neither variable is set — headless (CI container, server, ssh without X
    /// forwarding). Windowed tests must skip rather than fail.
    None,
}

impl DisplayServer {
    /// Classify a session from the two environment variables that indicate a
    /// graphical session on Linux.
    ///
    /// `WAYLAND_DISPLAY` wins when both are set, because a Wayland session also
    /// exports `DISPLAY` for XWayland clients, and the compositor is the more
    /// specific signal.
    #[must_use]
    pub fn detect(wayland_display: Option<&str>, display: Option<&str>) -> Self {
        let has_wayland = wayland_display.is_some_and(|v| !v.is_empty());
        let has_x11 = display.is_some_and(|v| !v.is_empty());
        if has_wayland {
            Self::Wayland
        } else if has_x11 {
            Self::X11
        } else {
            Self::None
        }
    }

    /// Whether a graphical session is available at all.
    #[must_use]
    pub const fn is_available(self) -> bool {
        !matches!(self, Self::None)
    }

    /// Reason string to print when a windowed test is skipped.
    ///
    /// A skipped test that prints why is honest; one that skips silently is
    /// indistinguishable from a test that never ran.
    #[must_use]
    pub const fn skip_reason(self) -> Option<&'static str> {
        match self {
            Self::None => Some(
                "no display server: neither WAYLAND_DISPLAY nor DISPLAY is set; \
                 windowed test skipped",
            ),
            _ => None,
        }
    }

    /// Short label for log output.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Wayland => "wayland",
            Self::X11 => "x11",
            Self::None => "headless",
        }
    }
}

/// Classify the current process's session.
#[must_use]
pub fn current_display_server() -> DisplayServer {
    let wayland = std::env::var("WAYLAND_DISPLAY").ok();
    let x11 = std::env::var("DISPLAY").ok();
    DisplayServer::detect(wayland.as_deref(), x11.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wayland_detected() {
        assert_eq!(
            DisplayServer::detect(Some("wayland-0"), None),
            DisplayServer::Wayland
        );
    }

    #[test]
    fn x11_detected() {
        assert_eq!(
            DisplayServer::detect(None, Some(":0")),
            DisplayServer::X11
        );
    }

    #[test]
    fn wayland_wins_when_both_present() {
        // A Wayland session exports DISPLAY for XWayland; the compositor is the
        // more specific signal and must win.
        assert_eq!(
            DisplayServer::detect(Some("wayland-1"), Some(":1")),
            DisplayServer::Wayland
        );
    }

    #[test]
    fn headless_when_neither_set() {
        assert_eq!(DisplayServer::detect(None, None), DisplayServer::None);
        assert!(!DisplayServer::None.is_available());
    }

    #[test]
    fn empty_vars_count_as_unset() {
        assert_eq!(
            DisplayServer::detect(Some(""), Some("")),
            DisplayServer::None,
            "empty strings must not be treated as a live session"
        );
    }

    #[test]
    fn skip_reason_only_for_headless() {
        assert!(DisplayServer::None.skip_reason().is_some());
        assert!(DisplayServer::Wayland.skip_reason().is_none());
        assert!(DisplayServer::X11.skip_reason().is_none());
    }

    #[test]
    fn availability_flags() {
        assert!(DisplayServer::Wayland.is_available());
        assert!(DisplayServer::X11.is_available());
        assert!(!DisplayServer::None.is_available());
    }
}
