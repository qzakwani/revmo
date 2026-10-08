//! Icon lookup following the freedesktop icon theme spec.
//!
//! https://specifications.freedesktop.org/icon-theme-spec/latest/

use std::cmp::Reverse;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::base_dirs::{config_home, current_desktops};
use super::keyfile::KeyFile;

const FALLBACK_THEME: &str = "hicolor";
// The spec's order is png, svg, xpm; iced can't render xpm.
const EXTENSIONS: [&str; 2] = ["png", "svg"];
const MAX_INHERIT_DEPTH: usize = 16;
// Desktops that store the icon theme in org.gnome.desktop.interface.
const GSETTINGS_DESKTOPS: [&str; 4] = ["gnome", "unity", "budgie", "pantheon"];

/// Icon search paths, in the order the spec gives:
/// `~/.icons`, `<data dir>/icons` for each data dir, then pixmaps.
pub fn base_dirs(home: &Path, data_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs = vec![home.join(".icons")];
    dirs.extend(data_dirs.iter().map(|d| d.join("icons")));
    dirs.extend(data_dirs.iter().map(|d| d.join("pixmaps")));
    dirs.retain(|d| d.is_dir());
    dirs
}

/// The user's configured icon theme, falling back to `hicolor`.
pub fn current_theme(home: &Path) -> String {
    let config = config_home(home);
    let desktops = current_desktops();
    let kde = || read_setting(&config.join("kdeglobals"), "Icons", "Theme");
    let gtk_ini = || {
        ["gtk-4.0", "gtk-3.0"].iter().find_map(|v| {
            read_setting(
                &config.join(v).join("settings.ini"),
                "Settings",
                "gtk-icon-theme-name",
            )
        })
    };
    let theme = if desktops.iter().any(|d| d == "kde") {
        // Plasma doesn't write the key while the default theme is in use.
        kde().or_else(|| Some("breeze".to_string()))
    } else if desktops
        .iter()
        .any(|d| GSETTINGS_DESKTOPS.contains(&d.as_str()))
    {
        gsettings_icon_theme().or_else(gtk_ini)
    } else {
        // Elsewhere gsettings usually just reports its built-in default.
        gtk_ini().or_else(gsettings_icon_theme).or_else(kde)
    };
    theme.unwrap_or_else(|| FALLBACK_THEME.to_string())
}

fn read_setting(path: &Path, group: &str, key: &str) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    KeyFile::parse(&contents)
        .group(group)?
        .string(key)
        .filter(|v| !v.is_empty())
}

fn gsettings_icon_theme() -> Option<String> {
    let output = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "icon-theme"])
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim().trim_matches('\'');
    (!value.is_empty()).then(|| value.to_string())
}

pub struct IconLookup {
    base_dirs: Vec<PathBuf>,
    /// The theme followed by its ancestors, ending with hicolor.
    themes: Vec<Theme>,
}

struct Theme {
    /// `<base dir>/<theme name>` for every base dir that has the theme.
    roots: Vec<PathBuf>,
    /// Subdirectories that match the requested size.
    exact: Vec<String>,
    /// The other subdirectories, closest size first.
    closest: Vec<String>,
}

#[derive(Debug, PartialEq)]
enum DirType {
    Fixed,
    Scalable,
    Threshold,
}

struct ThemeDir {
    name: String,
    size: u32,
    scale: u32,
    kind: DirType,
    min: u32,
    max: u32,
    threshold: u32,
}

impl ThemeDir {
    fn matches(&self, size: u32) -> bool {
        if self.scale != 1 {
            return false;
        }
        match self.kind {
            DirType::Fixed => self.size == size,
            DirType::Scalable => (self.min..=self.max).contains(&size),
            DirType::Threshold => (self.size.saturating_sub(self.threshold)
                ..=self.size + self.threshold)
                .contains(&size),
        }
    }

    fn distance(&self, size: u32) -> u32 {
        let (low, high) = match self.kind {
            DirType::Fixed => (self.size, self.size),
            DirType::Scalable => (self.min, self.max),
            DirType::Threshold => (
                self.size.saturating_sub(self.threshold),
                self.size + self.threshold,
            ),
        };
        let (low, high) = (low * self.scale, high * self.scale);
        if size < low {
            low - size
        } else {
            size.saturating_sub(high)
        }
    }
}

impl IconLookup {
    /// Loads `theme` and the themes it inherits from. `size` is the wanted
    /// icon size in pixels.
    pub fn new(theme: &str, base_dirs: Vec<PathBuf>, size: u32) -> Self {
        let mut names: Vec<String> = Vec::new();
        collect_theme_chain(theme, &base_dirs, &mut names, 0);
        if !names.iter().any(|n| n == FALLBACK_THEME) {
            names.push(FALLBACK_THEME.to_string());
        }
        let themes = names
            .iter()
            .filter_map(|name| load_theme(name, &base_dirs, size))
            .collect();
        IconLookup { base_dirs, themes }
    }

    /// Resolves an `Icon=` value to a file.
    pub fn find(&self, icon: &str) -> Option<PathBuf> {
        if icon.starts_with('/') {
            let path = PathBuf::from(icon);
            return path.is_file().then_some(path);
        }
        // Icon names shouldn't carry an extension, but some files do it anyway.
        let name = ["png", "svg", "xpm"]
            .iter()
            .find_map(|ext| icon.strip_suffix(&format!(".{ext}")))
            .unwrap_or(icon);
        if name.is_empty() || name.contains('/') {
            return None;
        }

        self.themes
            .iter()
            .find_map(|theme| {
                lookup_in(&theme.roots, &theme.exact, name)
                    .or_else(|| lookup_in(&theme.roots, &theme.closest, name))
            })
            .or_else(|| find_file(&self.base_dirs, name))
    }
}

fn lookup_in(roots: &[PathBuf], dirs: &[String], name: &str) -> Option<PathBuf> {
    dirs.iter().find_map(|dir| {
        let dirs: Vec<PathBuf> = roots.iter().map(|r| r.join(dir)).collect();
        find_file(&dirs, name)
    })
}

fn find_file(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    dirs.iter().find_map(|dir| {
        EXTENSIONS
            .iter()
            .map(|ext| dir.join(format!("{name}.{ext}")))
            .find(|p| p.is_file())
    })
}

/// Depth-first walk of `Inherits`, which is the order the spec's lookup visits themes.
fn collect_theme_chain(name: &str, base_dirs: &[PathBuf], out: &mut Vec<String>, depth: usize) {
    if depth > MAX_INHERIT_DEPTH || out.iter().any(|n| n == name) {
        return;
    }
    let Some(index) = read_index(name, base_dirs) else {
        return;
    };
    out.push(name.to_string());
    let parents = index
        .group("Icon Theme")
        .map(|g| g.list("Inherits", ','))
        .unwrap_or_default();
    for parent in parents {
        // hicolor is always searched last, not where a theme happens to list it.
        if parent != FALLBACK_THEME {
            collect_theme_chain(&parent, base_dirs, out, depth + 1);
        }
    }
}

fn read_index(name: &str, base_dirs: &[PathBuf]) -> Option<KeyFile> {
    base_dirs.iter().find_map(|dir| {
        let contents = fs::read_to_string(dir.join(name).join("index.theme")).ok()?;
        Some(KeyFile::parse(&contents))
    })
}

fn load_theme(name: &str, base_dirs: &[PathBuf], size: u32) -> Option<Theme> {
    let index = read_index(name, base_dirs)?;
    let main = index.group("Icon Theme")?;
    let mut dir_names = main.list("Directories", ',');
    dir_names.extend(main.list("ScaledDirectories", ','));

    let dirs: Vec<ThemeDir> = dir_names
        .into_iter()
        .filter_map(|dir| {
            let group = index.group(&dir)?;
            let size = group.number("Size")?;
            let kind = match group.raw("Type") {
                Some("Fixed") => DirType::Fixed,
                Some("Scalable") => DirType::Scalable,
                _ => DirType::Threshold,
            };
            Some(ThemeDir {
                size,
                scale: group.number("Scale").unwrap_or(1).max(1),
                kind,
                min: group.number("MinSize").unwrap_or(size),
                max: group.number("MaxSize").unwrap_or(size),
                threshold: group.number("Threshold").unwrap_or(2),
                name: dir,
            })
        })
        .collect();

    let (exact, mut closest): (Vec<_>, Vec<_>) = dirs.into_iter().partition(|d| d.matches(size));
    // On a tie prefer the bigger icon: scaling down looks better than scaling up.
    closest.sort_by_key(|d| (d.distance(size), Reverse(d.size * d.scale)));

    let roots: Vec<PathBuf> = base_dirs
        .iter()
        .map(|d| d.join(name))
        .filter(|d| d.is_dir())
        .collect();

    Some(Theme {
        roots,
        exact: exact.into_iter().map(|d| d.name).collect(),
        closest: closest.into_iter().map(|d| d.name).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    const HICOLOR: &str = "[Icon Theme]
Name=Hicolor
Directories=16x16/apps,32x32/apps,48x48/apps,scalable/apps

[16x16/apps]
Size=16
Type=Threshold

[32x32/apps]
Size=32
Type=Threshold

[48x48/apps]
Size=48
Type=Threshold

[scalable/apps]
Size=128
MinSize=8
MaxSize=512
Type=Scalable
";

    #[test]
    fn finds_icons_through_theme_chain() {
        let tmp = tempfile::tempdir().unwrap();
        let user = tmp.path().join("user/icons");
        let system = tmp.path().join("system/icons");
        let pixmaps = tmp.path().join("system/pixmaps");

        write(&system.join("hicolor/index.theme"), HICOLOR);
        write(
            &system.join("Child/index.theme"),
            "[Icon Theme]\nInherits=hicolor,Parent\nDirectories=apps/64\n\n[apps/64]\nSize=64\nType=Fixed\n",
        );
        write(
            &system.join("Parent/index.theme"),
            "[Icon Theme]\nDirectories=apps/32\n\n[apps/32]\nSize=32\nType=Fixed\n",
        );
        for file in [
            "hicolor/16x16/apps/firefox.png",
            "hicolor/48x48/apps/firefox.png",
            "hicolor/32x32/apps/firefox-developer-edition.png",
            "hicolor/scalable/apps/gimp.svg",
            "Child/apps/64/terminal.svg",
            "Parent/apps/32/terminal.svg",
            "Parent/apps/32/files.png",
        ] {
            write(&system.join(file), "");
        }
        // A theme can be spread over several base dirs.
        write(&user.join("hicolor/32x32/apps/editor.png"), "");
        write(&pixmaps.join("legacy.png"), "");
        write(&pixmaps.join("old.xpm"), "");
        write(&tmp.path().join("abs.svg"), "");

        let lookup = IconLookup::new(
            "Child",
            vec![user.clone(), system.clone(), pixmaps.clone()],
            32,
        );
        let find = |name: &str| lookup.find(name);

        // Child's own icon wins even though it's a worse size than Parent's.
        assert_eq!(
            find("terminal"),
            Some(system.join("Child/apps/64/terminal.svg"))
        );
        assert_eq!(find("files"), Some(system.join("Parent/apps/32/files.png")));
        // No 32px firefox; 16px and 48px are equally far, so the bigger one wins.
        assert_eq!(
            find("firefox"),
            Some(system.join("hicolor/48x48/apps/firefox.png"))
        );
        // A prefix of the name isn't a match.
        assert_eq!(find("firefox-developer"), None);
        assert_eq!(
            find("gimp"),
            Some(system.join("hicolor/scalable/apps/gimp.svg"))
        );
        assert_eq!(
            find("editor"),
            Some(user.join("hicolor/32x32/apps/editor.png"))
        );
        assert_eq!(find("legacy"), Some(pixmaps.join("legacy.png")));
        assert_eq!(find("legacy.png"), Some(pixmaps.join("legacy.png")));
        assert_eq!(find("old"), None);
        let abs = tmp.path().join("abs.svg");
        assert_eq!(find(abs.to_str().unwrap()), Some(abs.clone()));
        assert_eq!(find("/does/not/exist.png"), None);
    }

    #[test]
    fn falls_back_to_hicolor_for_unknown_theme() {
        let tmp = tempfile::tempdir().unwrap();
        write(&tmp.path().join("hicolor/index.theme"), HICOLOR);
        write(&tmp.path().join("hicolor/48x48/apps/app.png"), "");
        let lookup = IconLookup::new("Missing", vec![tmp.path().to_path_buf()], 32);
        assert_eq!(
            lookup.find("app"),
            Some(tmp.path().join("hicolor/48x48/apps/app.png"))
        );
    }

    #[test]
    fn inheritance_cycles_terminate() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            &tmp.path().join("A/index.theme"),
            "[Icon Theme]\nInherits=B\n",
        );
        write(
            &tmp.path().join("B/index.theme"),
            "[Icon Theme]\nInherits=A\n",
        );
        let mut chain = Vec::new();
        collect_theme_chain("A", &[tmp.path().to_path_buf()], &mut chain, 0);
        assert_eq!(chain, ["A", "B"]);
    }
}
