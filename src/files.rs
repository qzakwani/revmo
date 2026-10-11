//! Writing files so that a crash or kill part way through never leaves a
//! truncated file behind.

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Mutex;

/// Serializes writes within this process, which share one temp file name.
static WRITE_LOCK: Mutex<()> = Mutex::new(());

/// Replaces `path` with `bytes`, creating its directory if needed. Readers
/// see either the old contents or the new ones, never a mix: the bytes go
/// to a temp file next to `path`, which is synced and then renamed over it.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let (dir, name) = match (path.parent(), path.file_name()) {
        (Some(dir), Some(name)) => (dir, name),
        _ => {
            return Err(io::Error::other(format!(
                "not a file path: {}",
                path.display()
            )));
        }
    };
    let dir = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };
    fs::create_dir_all(dir)?;

    let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    remove_stale_temps(dir, name);
    let tmp = temp_path(dir, name, process::id());
    let result = File::create(&tmp)
        .and_then(|mut file| {
            file.write_all(bytes)?;
            // The data must be on disk before the rename makes it visible,
            // or a crash could leave `path` pointing at an empty file.
            file.sync_all()
        })
        .and_then(|_| fs::rename(&tmp, path));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result?;
    // Persist the rename itself. Failing here leaves a valid file either way.
    if let Ok(dir) = File::open(dir) {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// `.<name>.<pid>.tmp` in `dir`.
fn temp_path(dir: &Path, name: &std::ffi::OsStr, pid: u32) -> PathBuf {
    let mut tmp = OsString::from(".");
    tmp.push(name);
    tmp.push(format!(".{pid}.tmp"));
    dir.join(tmp)
}

/// Deletes temp files left by processes that were killed mid-write.
fn remove_stale_temps(dir: &Path, name: &std::ffi::OsStr) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let prefix = format!(".{}.", name.to_string_lossy());
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(pid) = file_name
            .to_str()
            .and_then(|f| f.strip_prefix(&prefix))
            .and_then(|f| f.strip_suffix(".tmp"))
            .and_then(|pid| pid.parse::<u32>().ok())
        else {
            continue;
        };
        // A live process may be writing it right now.
        if !Path::new(&format!("/proc/{pid}")).exists() {
            let _ = fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_replaces() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("a/b/file");
        write_atomic(&path, b"first").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"first");
        write_atomic(&path, b"second").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        // Nothing but the file itself is left in the directory.
        let names: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["file"]);
    }

    #[test]
    fn interrupted_write_keeps_old_contents_and_is_cleaned_up() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("file");
        write_atomic(&path, b"old").unwrap();

        // What a process killed between creating and renaming its temp file
        // leaves behind. No pid gets this high (pid_max is at most 2^22).
        let stale = temp_path(tmp.path(), "file".as_ref(), u32::MAX);
        fs::write(&stale, b"partial").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"old");

        write_atomic(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert!(!stale.exists());
    }

    #[test]
    fn leaves_temp_files_of_live_processes_and_other_files() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("file");
        // pid 1 is always running.
        let live = temp_path(tmp.path(), "file".as_ref(), 1);
        let other = tmp.path().join(".other.99999999.tmp");
        fs::write(&live, b"").unwrap();
        fs::write(&other, b"").unwrap();

        write_atomic(&path, b"data").unwrap();
        assert!(live.exists());
        assert!(other.exists());
    }
}
