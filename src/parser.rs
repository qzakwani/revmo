use std::fs::read_to_string;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::xdg::desktop_entry::{self, Context, Exec};
use crate::xdg::icons::{self, IconLookup};
use crate::xdg::{base_dirs, keyfile::Locale};
use crate::{store, HOME_DIR};

// Icons are drawn at styles::ICON_SIZE (30px). Asking for 64px keeps them
// sharp on 2x HiDPI screens; they scale down cleanly on 1x ones.
const ICON_LOOKUP_SIZE: u32 = 64;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct App {
    pub id: usize,
    pub name: String,
    pub exec: String,
    pub args: Option<String>,
    pub action: Option<String>,
    pub icon: Option<String>,
}

impl App {
    pub async fn get() -> Vec<App> {
        if let Some(apps) = store::get_apps() {
            return apps;
        }
        let mut apps = scan();
        apps.sort_by_cached_key(|a| a.name.to_lowercase());
        for (i, app) in apps.iter_mut().enumerate() {
            app.id = i;
        }
        store::store_apps(&apps);
        apps
    }

    fn new(name: &str, exec: Exec, action: Option<String>, icon: Option<String>) -> Self {
        App {
            id: 0,
            name: name.to_string(),
            exec: exec.command,
            args: (!exec.args.is_empty()).then(|| exec.args.join(",")),
            action,
            icon,
        }
    }
}

fn scan() -> Vec<App> {
    let home = Path::new(HOME_DIR.as_str());
    let data_dirs = base_dirs::data_dirs(home);
    let ctx = Context {
        locale: Locale::from_env(),
        desktops: base_dirs::current_desktops(),
    };
    let icons = IconLookup::new(
        &icons::current_theme(home),
        icons::base_dirs(home, &data_dirs),
        ICON_LOOKUP_SIZE,
    );

    let mut apps = Vec::new();
    for file in base_dirs::desktop_files(&data_dirs) {
        let Ok(contents) = read_to_string(&file.path) else {
            eprintln!("Error: can't read {}", file.path.display());
            continue;
        };
        let entry = match desktop_entry::parse(&contents, &file.path, &ctx) {
            Ok(entry) => entry,
            // Hidden, NoDisplay etc. are expected; nothing to report.
            Err(_) => continue,
        };

        let icon = entry.icon.as_deref().and_then(|i| icons.find(i));
        let icon = icon.map(|p| p.to_string_lossy().into_owned());
        apps.push(App::new(&entry.name, entry.exec, None, icon.clone()));
        for action in entry.actions {
            let action_icon = action
                .icon
                .as_deref()
                .and_then(|i| icons.find(i))
                .map(|p| p.to_string_lossy().into_owned())
                .or_else(|| icon.clone());
            apps.push(App::new(
                &entry.name,
                action.exec,
                Some(action.name),
                action_icon,
            ));
        }
    }
    apps
}
