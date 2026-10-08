//! XDG base directories and desktop file discovery.
//!
//! https://specifications.freedesktop.org/basedir-spec/latest/
//! https://specifications.freedesktop.org/desktop-entry-spec/latest/file-naming.html

use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_DATA_DIRS: &str = "/usr/local/share:/usr/share";
// Directories that are not always in $XDG_DATA_DIRS (e.g. when revmo is
// started from a hotkey daemon whose environment lacks the Flatpak/Snap
// profile scripts). They are searched last.
const FALLBACK_SYSTEM_DATA_DIRS: [&str; 2] =
    ["/var/lib/flatpak/exports/share", "/var/lib/snapd/desktop"];
const FALLBACK_USER_DATA_DIR: &str = ".local/share/flatpak/exports/share";
// Guards against symlink loops inside applications/.
const MAX_DEPTH: usize = 8;

/// Data directories in priority order: `$XDG_DATA_HOME` first, then
/// `$XDG_DATA_DIRS`, then the Flatpak/Snap fallbacks.
pub fn data_dirs(home: &Path) -> Vec<PathBuf> {
    data_dirs_from(
        home,
        env::var_os("XDG_DATA_HOME"),
        env::var_os("XDG_DATA_DIRS"),
    )
}

fn data_dirs_from(
    home: &Path,
    data_home: Option<OsString>,
    data_dirs: Option<OsString>,
) -> Vec<PathBuf> {
    // Relative paths are invalid per the spec and are ignored, which also
    // makes an empty variable fall back to its default.
    let data_home = data_home
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home.join(".local/share"));
    let data_dirs = data_dirs
        .filter(|v| env::split_paths(v).any(|p| p.is_absolute()))
        .unwrap_or_else(|| DEFAULT_DATA_DIRS.into());

    let mut dirs = vec![data_home];
    dirs.extend(env::split_paths(&data_dirs).filter(|p| p.is_absolute()));
    dirs.push(home.join(FALLBACK_USER_DATA_DIR));
    dirs.extend(FALLBACK_SYSTEM_DATA_DIRS.iter().map(PathBuf::from));

    let mut seen = HashSet::new();
    dirs.retain(|d| seen.insert(normalize(d)));
    dirs
}

fn normalize(p: &Path) -> PathBuf {
    p.components().collect()
}

/// A `.desktop` file together with its desktop file ID.
#[derive(Debug, PartialEq)]
pub struct DesktopFile {
    pub id: String,
    pub path: PathBuf,
}

/// Every `.desktop` file under `<data dir>/applications`. When the same
/// desktop ID exists in several data dirs, only the highest priority one is
/// returned, so user files override (or with `Hidden=true`, remove) system ones.
pub fn desktop_files(data_dirs: &[PathBuf]) -> Vec<DesktopFile> {
    let mut seen = HashSet::new();
    let mut files = Vec::new();
    for dir in data_dirs {
        let apps_dir = dir.join("applications");
        let mut found = Vec::new();
        collect_desktop_files(&apps_dir, &apps_dir, 0, &mut found);
        // read_dir order is unspecified; sort so results are reproducible.
        found.sort_by(|a, b| a.id.cmp(&b.id));
        for file in found {
            if seen.insert(file.id.clone()) {
                files.push(file);
            }
        }
    }
    files
}

fn collect_desktop_files(root: &Path, dir: &Path, depth: usize, out: &mut Vec<DesktopFile>) {
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // is_dir/is_file follow symlinks, which Flatpak exports rely on.
        if path.is_dir() {
            collect_desktop_files(root, &path, depth + 1, out);
        } else if path.extension().is_some_and(|e| e == "desktop") && path.is_file() {
            if let Some(id) = desktop_id(root, &path) {
                out.push(DesktopFile { id, path });
            }
        }
    }
}

/// `applications/kde/foo.desktop` has the ID `kde-foo.desktop`.
fn desktop_id(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let parts: Vec<_> = rel.iter().map(|p| p.to_str()).collect::<Option<_>>()?;
    Some(parts.join("-"))
}

/// Lowercased names from `$XDG_CURRENT_DESKTOP`, used for `OnlyShowIn`/`NotShowIn`.
pub fn current_desktops() -> Vec<String> {
    env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// `$XDG_CONFIG_HOME`, defaulting to `~/.config`.
pub fn config_home(home: &Path) -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home.join(".config"))
}

/// Whether `program` names an executable file, either as a path or via `$PATH`.
pub fn find_executable(program: &str) -> bool {
    let is_exec = |p: &Path| {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    };
    if program.contains('/') {
        return is_exec(Path::new(program));
    }
    env::var_os("PATH")
        .is_some_and(|paths| env::split_paths(&paths).any(|dir| is_exec(&dir.join(program))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dirs_defaults_and_order() {
        let home = Path::new("/home/u");
        let dirs = data_dirs_from(home, None, None);
        assert_eq!(
            dirs,
            [
                "/home/u/.local/share",
                "/usr/local/share",
                "/usr/share",
                "/home/u/.local/share/flatpak/exports/share",
                "/var/lib/flatpak/exports/share",
                "/var/lib/snapd/desktop",
            ]
            .map(PathBuf::from)
        );
    }

    #[test]
    fn data_dirs_from_env_skips_relative_and_duplicates() {
        let home = Path::new("/home/u");
        let dirs = data_dirs_from(
            home,
            Some("relative".into()),
            Some("/nix/share:rel/dir:/var/lib/flatpak/exports/share/".into()),
        );
        assert_eq!(
            dirs,
            [
                "/home/u/.local/share",
                "/nix/share",
                "/var/lib/flatpak/exports/share/",
                "/home/u/.local/share/flatpak/exports/share",
                "/var/lib/snapd/desktop",
            ]
            .map(PathBuf::from)
        );
    }

    #[test]
    fn desktop_files_use_ids_and_priority() {
        let tmp = tempfile::tempdir().unwrap();
        let user = tmp.path().join("user");
        let system = tmp.path().join("system");
        for (dir, file) in [
            (&user, "applications/firefox.desktop"),
            (&system, "applications/firefox.desktop"),
            (&system, "applications/kde/dolphin.desktop"),
            (&system, "applications/readme.txt"),
        ] {
            let path = dir.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }

        let files = desktop_files(&[user.clone(), system.clone(), tmp.path().join("missing")]);
        assert_eq!(
            files,
            [
                DesktopFile {
                    id: "firefox.desktop".into(),
                    path: user.join("applications/firefox.desktop"),
                },
                DesktopFile {
                    id: "kde-dolphin.desktop".into(),
                    path: system.join("applications/kde/dolphin.desktop"),
                },
            ]
        );
    }
}
