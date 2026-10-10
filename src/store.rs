//! Cache of scanned apps, so startup doesn't rescan every time.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::xdg::base_dirs::cache_home;
use crate::{parser::App, HOME_DIR};

// Bump when App or scanning changes, so old caches are rebuilt.
const VERSION: u32 = 2;

static CACHE_FILE: LazyLock<PathBuf> =
    LazyLock::new(|| cache_home(Path::new(HOME_DIR.as_str())).join("revmo/apps"));

// Files earlier versions kept in ~/.local/share/revmo.
const LEGACY_FILES: [&str; 3] = ["apps.store", "apps.v2.store", "apps.v3.store"];

#[derive(Serialize, Deserialize)]
struct Cache {
    version: u32,
    /// Identifies the desktop files and environment the apps were scanned from.
    fingerprint: u64,
    apps: Vec<App>,
}

/// The cached apps, if they were scanned with the same `fingerprint`.
pub fn get_apps(fingerprint: u64) -> Option<Vec<App>> {
    for legacy in LEGACY_FILES {
        let _ = fs::remove_file(format!("{}/.local/share/revmo/{legacy}", *HOME_DIR));
    }
    let bytes = fs::read(&*CACHE_FILE).ok()?;
    let cache: Cache = bincode::deserialize(&bytes).ok()?;
    (cache.version == VERSION && cache.fingerprint == fingerprint && !cache.apps.is_empty())
        .then_some(cache.apps)
}

pub fn store_apps(fingerprint: u64, apps: &[App]) {
    let cache = Cache {
        version: VERSION,
        fingerprint,
        apps: apps.to_vec(),
    };
    let result = bincode::serialize(&cache)
        .map_err(std::io::Error::other)
        .and_then(|bytes| {
            if let Some(dir) = CACHE_FILE.parent() {
                fs::create_dir_all(dir)?;
            }
            fs::write(&*CACHE_FILE, bytes)
        });
    if let Err(e) = result {
        eprintln!("Failed to store apps: {e}");
    }
}
