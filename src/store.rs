use once_cell::sync::Lazy;

use crate::{parser::App, HOME_DIR};

static APP_STORE: Lazy<String> =
    Lazy::new(|| format!("{}/.local/share/revmo/apps.store", *HOME_DIR));

pub fn get_apps() -> Option<Vec<App>> {
    if let Ok(v) = std::fs::read(&*APP_STORE) {
        if let Ok(apps) = bincode::deserialize::<Vec<App>>(&v) {
            if apps.is_empty() {
                return None;
            } else {
                return Some(apps);
            }
        }
    }
    None
}

pub fn store_apps(apps: &[App]) {
    let r = std::fs::write(&*APP_STORE, bincode::serialize(apps).unwrap_or(vec![]));
    if r.is_err() {
        eprintln!("Failed to store apps");
    }
}

pub fn empty_app_store() -> Option<()> {
    let r = std::fs::write(&*APP_STORE, vec![]);
    if r.is_err() {
        eprintln!("Failed to empty app store");
        return None;
    }
    Some(())
}
