use std::fs;

use bincode;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::HOME_DIR;

pub const DEFAULT_THEME: usize = 0;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub theme_id: usize,
}

static CONFIG_FILE: Lazy<String> = Lazy::new(|| format!("{}/.config/revmo/revmo.conf", *HOME_DIR));

impl Config {
    fn default() -> Self {
        Config {
            theme_id: DEFAULT_THEME,
        }
    }

    pub fn load() -> Self {
        match fs::read(&*CONFIG_FILE) {
            Ok(v) => bincode::deserialize(&v).unwrap_or(Config::default()),
            Err(e) => {
                let def = Config::default();
                if e.kind() == std::io::ErrorKind::NotFound {
                    def.save();
                } else {
                    eprintln!("Failed to load config file: {}", e);
                }
                def
            }
        }
    }
    pub fn save(&self) {
        let r = fs::write(&*CONFIG_FILE, bincode::serialize(self).unwrap_or(vec![]));
        if r.is_err() {
            eprintln!("Failed to save config file");
        }
    }

    pub fn reset() -> Option<()> {
        let r = fs::write(
            &*CONFIG_FILE,
            bincode::serialize(&Config::default()).unwrap_or(vec![]),
        );
        if r.is_err() {
            eprintln!("Failed to reset config file");
            return None;
        }
        Some(())
    }
}
