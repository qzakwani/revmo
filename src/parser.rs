use std::env;
use std::fs::read_to_string;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;
use std::path::Path;

use rkyv::{Archive, Deserialize, Serialize};

use crate::{HOME_DIR, store};
use revmo::launch;
use revmo::search::Haystack;
use revmo::xdg::desktop_entry::{self, Context};
use revmo::xdg::exec::{Exec, FieldCode, split_user_args};
use revmo::xdg::icons::{self, IconLookup};
use revmo::xdg::{base_dirs, keyfile::Locale};

// Icons are drawn at styles::ICON_SIZE (30px). Asking for 64px keeps them
// sharp on 2x HiDPI screens; they scale down cleanly on 1x ones.
const ICON_LOOKUP_SIZE: u32 = 64;

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
pub struct App {
    /// Stable identity across rescans: the desktop file ID, plus
    /// `/<action id>` for actions.
    pub key: String,
    pub name: String,
    pub action: Option<String>,
    pub generic_name: Option<String>,
    pub keywords: Vec<String>,
    pub icon: Option<String>,
    pub exec: Exec,
    pub terminal: bool,
    pub working_dir: Option<String>,
}

impl App {
    /// The apps from the last scan and the fingerprint they were scanned
    /// with, without checking that they're still current.
    pub fn cached() -> Option<(u64, Vec<App>)> {
        store::get_apps()
    }

    /// A fresh scan if the desktop files or environment changed since
    /// `fingerprint` (or there is none), else `None`. Blocks on file IO.
    pub fn scan_if_changed(fingerprint: Option<u64>) -> Option<Vec<App>> {
        let home = Path::new(HOME_DIR.as_str());
        let data_dirs = base_dirs::data_dirs(home);
        let current = self::fingerprint(&data_dirs);
        (fingerprint != Some(current)).then(|| scan_and_store(home, &data_dirs, current))
    }

    /// A fresh scan, whether or not anything changed. Blocks on file IO.
    pub fn scan() -> Vec<App> {
        let home = Path::new(HOME_DIR.as_str());
        let data_dirs = base_dirs::data_dirs(home);
        let current = fingerprint(&data_dirs);
        scan_and_store(home, &data_dirs, current)
    }

    /// What the search box matches against, with each text's weight.
    pub fn haystack(&self) -> Haystack {
        let program = self.exec.program();
        let mut fields = vec![(self.name.as_str(), 1.0)];
        if let Some(action) = &self.action {
            fields.push((action.as_str(), 0.9));
        }
        fields.push((program.as_str(), 0.8));
        if let Some(generic) = &self.generic_name {
            fields.push((generic.as_str(), 0.7));
        }
        fields.extend(self.keywords.iter().map(|k| (k.as_str(), 0.6)));
        Haystack::new(fields)
    }

    /// The file/URL arguments the app takes, in order.
    pub fn arg_codes(&self) -> Vec<FieldCode> {
        self.exec.codes()
    }

    /// Starts the app. `inputs` holds what the user typed for each of
    /// [`App::arg_codes`].
    pub fn launch(&self, inputs: &[String]) -> io::Result<()> {
        let values: Vec<Vec<String>> = self
            .arg_codes()
            .iter()
            .zip(inputs)
            .map(|(code, input)| match input.trim() {
                "" => Vec::new(),
                input if code.is_list() => split_user_args(input),
                input => vec![input.to_string()],
            })
            .collect();
        launch::spawn(
            self.exec.expand(&values),
            self.terminal,
            self.working_dir.as_deref(),
        )
    }
}

/// Changes whenever a desktop file is added, removed or edited, or the
/// environment that decides how they're read changes. Icon theme changes
/// aren't tracked; Ctrl+R rescans.
fn fingerprint(data_dirs: &[std::path::PathBuf]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for var in ["XDG_CURRENT_DESKTOP", "LC_ALL", "LC_MESSAGES", "LANG"] {
        env::var_os(var).hash(&mut hasher);
    }
    data_dirs.hash(&mut hasher);
    base_dirs::applications_mtimes(data_dirs).hash(&mut hasher);
    hasher.finish()
}

/// Scans with the fingerprint taken before scanning, so a file changed
/// mid-scan triggers another scan next time.
fn scan_and_store(home: &Path, data_dirs: &[std::path::PathBuf], fingerprint: u64) -> Vec<App> {
    let mut apps = scan(home, data_dirs);
    apps.sort_by_cached_key(|a| a.name.to_lowercase());
    store::store_apps(fingerprint, &apps);
    apps
}

fn scan(home: &Path, data_dirs: &[std::path::PathBuf]) -> Vec<App> {
    let ctx = Context {
        locale: Locale::from_env(),
        desktops: base_dirs::current_desktops(),
    };
    let icons = IconLookup::new(
        &icons::current_theme(home),
        icons::base_dirs(home, data_dirs),
        ICON_LOOKUP_SIZE,
    );

    let mut apps = Vec::new();
    for file in base_dirs::desktop_files(data_dirs) {
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
        let app = |key: String, action: Option<String>, exec: Exec, icon: Option<String>| App {
            key,
            name: entry.name.clone(),
            action,
            generic_name: entry.generic_name.clone(),
            keywords: entry.keywords.clone(),
            icon,
            exec,
            terminal: entry.terminal,
            working_dir: entry.working_dir.clone(),
        };
        apps.push(app(file.id.clone(), None, entry.exec.clone(), icon.clone()));
        for action in &entry.actions {
            let action_icon = action
                .icon
                .as_deref()
                .and_then(|i| icons.find(i))
                .map(|p| p.to_string_lossy().into_owned())
                .or_else(|| icon.clone());
            apps.push(app(
                format!("{}/{}", file.id, action.id),
                Some(action.name.clone()),
                action.exec.clone(),
                action_icon,
            ));
        }
    }
    apps
}
