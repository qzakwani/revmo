//! Single-instance handling: starting revmo while it's already open closes
//! the open window instead, so one hotkey toggles it.

use std::env;
use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

pub enum Instance {
    /// No other instance was running; this one listens for later ones.
    Primary(UnixListener),
    /// Another instance was running and has been asked to close.
    Secondary,
}

/// `$XDG_RUNTIME_DIR/revmo.sock`, or a file in `fallback_dir` when there
/// is no runtime dir.
pub fn socket_path(fallback_dir: &Path) -> PathBuf {
    env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| fallback_dir.to_path_buf())
        .join("revmo.sock")
}

/// Becomes the primary instance, or tells the running one to close.
/// Returns `None` if the socket can't be set up; revmo then runs without
/// single-instance handling.
pub fn acquire(path: &Path) -> Option<Instance> {
    if let Ok(mut stream) = UnixStream::connect(path) {
        // A failed write still means someone is listening; don't open a
        // second window either way.
        let _ = stream.write_all(b"close\n");
        return Some(Instance::Secondary);
    }
    // Nobody answered: any file there is left over from an instance that
    // exited.
    let _ = std::fs::remove_file(path);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match UnixListener::bind(path) {
        Ok(listener) => Some(Instance::Primary(listener)),
        Err(e) => {
            eprintln!("Failed to create {}: {e}", path.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_instance_signals_the_first() {
        let _guard = crate::fork_lock();
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("run/revmo.sock");

        let Some(Instance::Primary(listener)) = acquire(&path) else {
            panic!("first instance should be primary");
        };
        assert!(matches!(acquire(&path), Some(Instance::Secondary)));
        assert!(listener.accept().is_ok());

        // Once the primary is gone, its leftover socket file is replaced.
        drop(listener);
        assert!(path.exists());
        assert!(matches!(acquire(&path), Some(Instance::Primary(_))));
    }
}
