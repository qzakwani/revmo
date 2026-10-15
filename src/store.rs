//! Caches of scanned apps and themes, so startup doesn't rescan every time.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use rkyv::rancor::Error;
use rkyv::util::AlignedVec;
use rkyv::{Archive, Deserialize, Serialize};

use crate::theme::ThemeDef;
use crate::{APP_DIR, parser::App};
use revmo::files::write_atomic;

// Bump when App or scanning changes, so old caches are rebuilt.
const VERSION: u32 = 2;
// Bump when ThemeDef or theme parsing changes.
const THEMES_VERSION: u32 = 2;

static CACHE_FILE: LazyLock<PathBuf> = LazyLock::new(|| APP_DIR.join("cache/apps"));
static THEMES_FILE: LazyLock<PathBuf> = LazyLock::new(|| APP_DIR.join("cache/themes"));

#[derive(Archive, Serialize, Deserialize)]
struct Cache {
    version: u32,
    /// Identifies the desktop files and environment the apps were scanned from.
    fingerprint: u64,
    apps: Vec<App>,
}

#[derive(Archive, Serialize, Deserialize)]
struct ThemeCache {
    version: u32,
    /// Identifies the theme files the themes were read from.
    fingerprint: u64,
    themes: Vec<ThemeDef>,
}

/// The cached apps and the fingerprint they were scanned with.
pub fn get_apps() -> Option<(u64, Vec<App>)> {
    read(&CACHE_FILE)
}

pub fn store_apps(fingerprint: u64, apps: &[App]) {
    if let Err(e) = write(&CACHE_FILE, fingerprint, apps) {
        eprintln!("Failed to store apps: {e}");
    }
}

/// The cached theme files' themes and the fingerprint they were read with.
pub fn get_themes() -> Option<(u64, Vec<ThemeDef>)> {
    read_themes(&THEMES_FILE)
}

pub fn store_themes(fingerprint: u64, themes: &[ThemeDef]) {
    if let Err(e) = write_themes(&THEMES_FILE, fingerprint, themes) {
        eprintln!("Failed to store themes: {e}");
    }
}

/// rkyv reads the data in place, so the buffer must be aligned for it.
fn read_bytes(path: &Path) -> Option<AlignedVec<16>> {
    let mut bytes = AlignedVec::<16>::new();
    bytes.extend_from_reader(&mut File::open(path).ok()?).ok()?;
    Some(bytes)
}

fn read(path: &Path) -> Option<(u64, Vec<App>)> {
    let bytes = read_bytes(path)?;
    // Validated, so a corrupt cache or one in an older format is rejected.
    let cache = rkyv::access::<ArchivedCache, Error>(&bytes).ok()?;
    if cache.version != VERSION || cache.apps.is_empty() {
        return None;
    }
    let apps = rkyv::deserialize::<Vec<App>, Error>(&cache.apps).ok()?;
    Some((cache.fingerprint.to_native(), apps))
}

fn write(path: &Path, fingerprint: u64, apps: &[App]) -> std::io::Result<()> {
    let cache = Cache {
        version: VERSION,
        fingerprint,
        apps: apps.to_vec(),
    };
    let bytes = rkyv::to_bytes::<Error>(&cache).map_err(std::io::Error::other)?;
    // Atomic, so quitting mid-write can't leave a cache that fails to load.
    write_atomic(path, &bytes)
}

fn read_themes(path: &Path) -> Option<(u64, Vec<ThemeDef>)> {
    let bytes = read_bytes(path)?;
    let cache = rkyv::access::<ArchivedThemeCache, Error>(&bytes).ok()?;
    if cache.version != THEMES_VERSION {
        return None;
    }
    let themes = rkyv::deserialize::<Vec<ThemeDef>, Error>(&cache.themes).ok()?;
    Some((cache.fingerprint.to_native(), themes))
}

fn write_themes(path: &Path, fingerprint: u64, themes: &[ThemeDef]) -> std::io::Result<()> {
    let cache = ThemeCache {
        version: THEMES_VERSION,
        fingerprint,
        themes: themes.to_vec(),
    };
    let bytes = rkyv::to_bytes::<Error>(&cache).map_err(std::io::Error::other)?;
    write_atomic(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use revmo::xdg::exec::{EntryInfo, Exec};

    fn app(key: &str, exec: &str) -> App {
        let info = EntryInfo {
            name: key,
            icon: None,
            path: Path::new("/x.desktop"),
        };
        App {
            key: key.into(),
            name: key.into(),
            action: None,
            generic_name: Some("Generic".into()),
            keywords: vec!["a".into(), "b".into()],
            icon: Some("/icon.png".into()),
            exec: Exec::parse(exec, &info).unwrap(),
            terminal: false,
            working_dir: None,
        }
    }

    #[test]
    fn round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("apps");
        let apps = [app("a.desktop", "a %U"), app("b.desktop/new", "b --new %f")];
        write(&path, 42, &apps).unwrap();
        let (fingerprint, read_apps) = read(&path).unwrap();
        assert_eq!(fingerprint, 42);
        assert_eq!(read_apps.len(), 2);
        for (a, b) in apps.iter().zip(&read_apps) {
            assert_eq!(
                (&a.key, &a.keywords, &a.exec),
                (&b.key, &b.keywords, &b.exec)
            );
            assert_eq!((&a.generic_name, &a.icon), (&b.generic_name, &b.icon));
        }
    }

    #[test]
    fn round_trips_themes() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("themes");
        let themes = crate::theme::built_in();
        write_themes(&path, 7, &themes).unwrap();
        assert_eq!(read_themes(&path), Some((7, themes.to_vec())));
        // No theme files is a valid, cacheable result.
        write_themes(&path, 8, &[]).unwrap();
        assert_eq!(read_themes(&path), Some((8, Vec::new())));
        std::fs::write(&path, [0xff; 64]).unwrap();
        assert_eq!(read_themes(&path), None);
    }

    #[test]
    fn rejects_bad_caches() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("apps");
        assert!(read(&path).is_none());

        // Empty lists are rescanned rather than trusted.
        write(&path, 1, &[]).unwrap();
        assert!(read(&path).is_none());

        // Truncated, garbage, and a bincode cache from an earlier version.
        write(&path, 1, &[app("a.desktop", "a")]).unwrap();
        let good = std::fs::read(&path).unwrap();
        let mut bincode_v3 = vec![3, 0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0];
        bincode_v3.extend_from_slice(&[9, 0, 0, 0, 0, 0, 0, 0]);
        bincode_v3.extend_from_slice(b"a.desktop");
        for bytes in [
            &good[..good.len() / 2],
            &[0xff; 64][..],
            &bincode_v3[..],
            &[],
        ] {
            std::fs::write(&path, bytes).unwrap();
            assert!(read(&path).is_none());
        }
    }
}
