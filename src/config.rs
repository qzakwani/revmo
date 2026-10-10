//! User settings, stored as a small text file the user can also edit:
//!
//! ```text
//! [Revmo]
//! Theme=Dracula
//! Hidden=org.gnome.Logs.desktop;firefox.desktop/new-private-window;
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::theme;
use crate::HOME_DIR;
use revmo::xdg::base_dirs::config_home;
use revmo::xdg::keyfile::{escape, KeyFile};

const GROUP: &str = "Revmo";

static CONFIG_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| config_home(Path::new(HOME_DIR.as_str())).join("revmo"));

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Theme name, as shown on the help page.
    pub theme: String,
    /// Keys of apps the user removed from the list.
    pub hidden: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: theme::default_theme().to_string(),
            hidden: Vec::new(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = CONFIG_DIR.join("config");
        if let Ok(contents) = fs::read_to_string(&path) {
            return Config::parse(&contents);
        }
        let config = migrate_legacy(&CONFIG_DIR.join("revmo.conf")).unwrap_or_default();
        config.save();
        config
    }

    pub fn save(&self) {
        let path = CONFIG_DIR.join("config");
        let result =
            fs::create_dir_all(&*CONFIG_DIR).and_then(|_| fs::write(&path, self.serialize()));
        if let Err(e) = result {
            eprintln!("Failed to save {}: {e}", path.display());
        }
    }

    fn parse(contents: &str) -> Self {
        let file = KeyFile::parse(contents);
        let Some(group) = file.group(GROUP) else {
            return Config::default();
        };
        Config {
            theme: group
                .string("Theme")
                .unwrap_or_else(|| Config::default().theme),
            hidden: group.list("Hidden", ';'),
        }
    }

    fn serialize(&self) -> String {
        let hidden: String = self
            .hidden
            .iter()
            .map(|k| format!("{};", escape(k, Some(';'))))
            .collect();
        format!(
            "[{GROUP}]\nTheme={}\nHidden={hidden}\n",
            escape(&self.theme, None)
        )
    }
}

/// Earlier versions stored a bincode-encoded theme index in `revmo.conf`.
fn migrate_legacy(path: &Path) -> Option<Config> {
    let bytes = fs::read(path).ok()?;
    let _ = fs::remove_file(path);
    let index: u64 = bincode::deserialize(&bytes).ok()?;
    let theme = theme::THEMES.get(usize::try_from(index).ok()?)?;
    Some(Config {
        theme: theme.to_string(),
        ..Config::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let config = Config {
            theme: "Nude Dracula".into(),
            hidden: vec!["a.desktop".into(), "b.desktop/new;window".into()],
        };
        assert_eq!(Config::parse(&config.serialize()), config);
    }

    #[test]
    fn missing_keys_use_defaults() {
        assert_eq!(Config::parse("[Revmo]\n"), Config::default());
        assert_eq!(Config::parse("garbage"), Config::default());
    }

    #[test]
    fn migrates_theme_index() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("revmo.conf");
        fs::write(&path, bincode::serialize(&4u64).unwrap()).unwrap();
        let config = migrate_legacy(&path).unwrap();
        assert_eq!(config.theme, theme::THEMES[4].to_string());
        assert!(!path.exists());

        fs::write(&path, bincode::serialize(&9999u64).unwrap()).unwrap();
        assert_eq!(migrate_legacy(&path), None);
    }
}
