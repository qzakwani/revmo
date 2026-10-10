//! Revmo's logic that doesn't depend on the UI: reading desktop entries
//! and icon themes, launching programs, search ranking and launch history.

pub mod instance;
pub mod launch;
pub mod search;
pub mod usage;
pub mod xdg;

/// Held by tests that start processes and by tests that check when a socket
/// stops accepting connections. A forked child holds a copy of every open
/// file descriptor until it execs, which keeps a dropped listener reachable
/// for a moment; tests run in parallel threads of one process, so the two
/// kinds must not overlap.
#[cfg(test)]
static FORK_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
fn fork_lock() -> std::sync::MutexGuard<'static, ()> {
    // A test that panicked while holding the lock has already failed.
    FORK_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}
