//! Main-thread guard (FR-011, FR-012).
//!
//! The first thread to call `MainThreadGuard::init_current()` wins the role of
//! main thread for the rest of the process. All subsequent `check()` calls
//! from any other thread return `Err(RnError::WrongThread)`.

use std::sync::OnceLock;
use std::thread::ThreadId;

use crate::error::RnError;

static MAIN_THREAD: OnceLock<ThreadId> = OnceLock::new();

/// Zero-sized marker type carrying the main-thread rule.
pub struct MainThreadGuard;

impl MainThreadGuard {
    /// Claim the current thread as the main thread. Idempotent — first winner
    /// stays the main thread for the process lifetime.
    pub fn init_current() {
        let _ = MAIN_THREAD.set(std::thread::current().id());
    }

    /// Return `Ok(())` if the calling thread is the main thread, else
    /// `Err(RnError::WrongThread)`.
    pub fn check() -> Result<(), RnError> {
        let main = MAIN_THREAD.get_or_init(|| std::thread::current().id());
        if *main == std::thread::current().id() {
            Ok(())
        } else {
            Err(RnError::WrongThread)
        }
    }

    /// Only for tests: a placeholder for forcing main-thread id reset. We
    /// can't reset `OnceLock`; tests should instead rely on the fact that the
    /// first thread to call `check()` becomes the main thread.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn reset_for_test() {}
}
