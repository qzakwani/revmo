use std::collections::HashMap;
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

    /// A fresh scan if the desktop files, icon theme or environment changed
    /// since `fingerprint` (or there is none), else `None`. Blocks on file
    /// IO, and on gsettings where that holds the icon theme.
    pub fn scan_if_changed(fingerprint: Option<u64>) -> Option<Vec<App>> {
        let home = Path::new(HOME_DIR.as_str());
        let data_dirs = base_dirs::data_dirs(home);
        let icon_theme = icons::current_theme(home);
        let current = self::fingerprint(&data_dirs, &icon_theme);
        (fingerprint != Some(current))
            .then(|| scan_and_store(home, &data_dirs, &icon_theme, current))
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

    /// For an action, the key of the app it belongs to.
    pub fn parent_key(&self) -> Option<&str> {
        self.action.as_ref()?;
        self.key.split_once('/').map(|(parent, _)| parent)
    }

    /// Whether the config's `Hidden=` list names it, or the app it belongs to.
    pub fn is_hidden(&self, hidden: &[String]) -> bool {
        let listed = |key: &str| hidden.iter().any(|h| h == key);
        listed(&self.key) || self.parent_key().is_some_and(listed)
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

/// For each of `apps`, the index of the app it's shown under: an action's
/// app, or itself. Actions whose app isn't in the list stand on their own.
pub fn parents(apps: &[App]) -> Vec<usize> {
    let index: HashMap<&str, usize> = apps
        .iter()
        .enumerate()
        .filter(|(_, a)| a.action.is_none())
        .map(|(i, a)| (a.key.as_str(), i))
        .collect();
    apps.iter()
        .enumerate()
        .map(|(i, a)| {
            a.parent_key()
                .and_then(|p| index.get(p).copied())
                .unwrap_or(i)
        })
        .collect()
}

/// The apps to show, best first, given each entry's score (`None` if it
/// doesn't match): an app scores as well as its best matching entry, so an
/// action's match shows its app. Ties keep the order of `scores`.
pub fn rank_groups(scores: &[Option<f64>], parents: &[usize]) -> Vec<usize> {
    let mut best: Vec<Option<f64>> = vec![None; scores.len()];
    for (i, score) in scores.iter().enumerate() {
        if let Some(score) = *score {
            let group = &mut best[parents[i]];
            *group = Some(group.map_or(score, |b: f64| b.max(score)));
        }
    }
    let mut ranked: Vec<(f64, usize)> = best
        .into_iter()
        .enumerate()
        .filter_map(|(i, s)| s.map(|s| (s, i)))
        .collect();
    // Stable, so ties keep the frecency order of the apps.
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    ranked.into_iter().map(|(_, i)| i).collect()
}

/// Changes whenever a desktop file is added, removed or edited, the icon
/// theme is switched, or the environment that decides how they're read
/// changes. Icons added to or removed from a theme aren't tracked.
fn fingerprint(data_dirs: &[std::path::PathBuf], icon_theme: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    for var in ["XDG_CURRENT_DESKTOP", "LC_ALL", "LC_MESSAGES", "LANG"] {
        env::var_os(var).hash(&mut hasher);
    }
    data_dirs.hash(&mut hasher);
    icon_theme.hash(&mut hasher);
    base_dirs::applications_mtimes(data_dirs).hash(&mut hasher);
    hasher.finish()
}

/// Scans with the fingerprint taken before scanning, so a file changed
/// mid-scan triggers another scan next time.
fn scan_and_store(
    home: &Path,
    data_dirs: &[std::path::PathBuf],
    icon_theme: &str,
    fingerprint: u64,
) -> Vec<App> {
    let mut apps = scan(home, data_dirs, icon_theme);
    apps.sort_by_cached_key(|a| a.name.to_lowercase());
    store::store_apps(fingerprint, &apps);
    apps
}

fn scan(home: &Path, data_dirs: &[std::path::PathBuf], icon_theme: &str) -> Vec<App> {
    let ctx = Context {
        locale: Locale::from_env(),
        desktops: base_dirs::current_desktops(),
    };
    let icons = IconLookup::new(
        icon_theme,
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

#[cfg(test)]
mod tests {
    use super::*;
    use revmo::xdg::exec::EntryInfo;

    fn app(key: &str, action: Option<&str>) -> App {
        let info = EntryInfo {
            name: key,
            icon: None,
            path: Path::new("/x.desktop"),
        };
        App {
            key: key.into(),
            name: key.into(),
            action: action.map(Into::into),
            generic_name: None,
            keywords: Vec::new(),
            icon: None,
            exec: Exec::parse("x", &info).unwrap(),
            terminal: false,
            working_dir: None,
        }
    }

    #[test]
    fn groups_actions_under_their_app() {
        let apps = [
            app("firefox.desktop/private", Some("Private")),
            app("vim.desktop", None),
            app("firefox.desktop", None),
            app("gone.desktop/new", Some("New")),
        ];
        assert_eq!(parents(&apps), [2, 1, 2, 3]);

        // Only Firefox's action matches: Firefox shows, above a weaker Vim.
        let scores = [Some(50.0), Some(10.0), None, None];
        assert_eq!(rank_groups(&scores, &parents(&apps)), [2, 1]);
        // The app's best entry counts; ties keep the list order.
        let scores = [Some(5.0), Some(10.0), Some(10.0), Some(1.0)];
        assert_eq!(rank_groups(&scores, &parents(&apps)), [1, 2, 3]);
    }

    #[test]
    fn hiding_an_app_hides_its_actions() {
        let hidden = vec!["firefox.desktop".to_string()];
        assert!(app("firefox.desktop", None).is_hidden(&hidden));
        assert!(app("firefox.desktop/private", Some("Private")).is_hidden(&hidden));
        assert!(!app("vim.desktop", None).is_hidden(&hidden));
        // A key with a slash but no action is not an action.
        assert!(!app("firefox.desktop/x", None).is_hidden(&hidden));
        // Older configs could hide a single action.
        let hidden = vec!["firefox.desktop/private".to_string()];
        assert!(app("firefox.desktop/private", Some("Private")).is_hidden(&hidden));
        assert!(!app("firefox.desktop", None).is_hidden(&hidden));
    }
}
