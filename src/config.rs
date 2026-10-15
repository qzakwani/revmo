//! User settings: a small text file the user writes (and can keep with
//! their dotfiles). Every setting has a default here; the file only
//! overrides the ones it sets. Revmo only writes `Theme=` (see
//! [`save_theme`]), which creates the file if there's none.
//!
//! ```text
//! [Revmo]
//! Theme=Dracula
//! Font=IBM Plex Mono
//! Scale=1.5
//! Hidden=org.gnome.Logs.desktop;firefox.desktop/new-private-window;
//! ```
//!
//! An empty `Font` uses the bundled IBM Plex Mono; an empty `Scale` picks
//! one from the monitor (see `revmo::display`). Mistakes are reported as
//! [`Loaded::problems`] and the setting keeps its default.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::HOME_DIR;
use crate::theme;
use revmo::files::write_atomic;
use revmo::xdg::base_dirs::config_home;
use revmo::xdg::keyfile::{KeyFile, escape};

const GROUP: &str = "Revmo";
const KEYS: [&str; 4] = ["Theme", "Font", "Scale", "Hidden"];

static CONFIG_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| config_home(Path::new(HOME_DIR.as_str())).join("revmo"));

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Theme name, as shown on the help page.
    pub theme: String,
    /// Font family name; empty for the bundled one.
    pub font: String,
    /// How much to enlarge the whole window; `None` to pick automatically.
    pub scale: Option<f32>,
    /// Desktop file IDs of apps to leave out of the list (with their
    /// actions); set by hand, revmo only reads it.
    pub hidden: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: theme::DEFAULT.to_string(),
            font: String::new(),
            scale: None,
            hidden: Vec::new(),
        }
    }
}

/// The settings in effect, and what was wrong with the file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Loaded {
    pub config: Config,
    /// One line per mistake, e.g. "line 3: unknown setting Them".
    pub problems: Vec<String>,
    /// Set when writing the file would lose what's in it (it couldn't be
    /// read or understood), with the reason.
    pub unsaveable: Option<String>,
}

impl Config {
    /// The settings from the config file over the defaults. A missing file
    /// just means defaults.
    pub fn load() -> Loaded {
        load_from(&CONFIG_DIR.join("config"))
    }
}

/// Saves `theme` as the config's `Theme=`, the only setting revmo writes.
/// The rest of the file stays as the user wrote it.
pub fn save_theme(theme: &str) -> io::Result<()> {
    save_theme_to(&CONFIG_DIR.join("config"), theme)
}

/// Reads the file afresh, so edits made while revmo is open are kept, and
/// refuses to write over a file it can't make sense of.
fn save_theme_to(path: &Path, theme: &str) -> io::Result<()> {
    let text = match fs::read(path) {
        Ok(bytes) => String::from_utf8(bytes)
            .map_err(|_| io::Error::other("the file isn't valid UTF-8 text"))?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    if let Some(reason) = parse(&text).unsaveable {
        return Err(io::Error::other(reason));
    }
    let text = with_theme(&text, theme);
    match fs::canonicalize(path) {
        // Atomic, so a crash can't leave the file half-written; to the
        // file a symlink points to, so a dotfiles link stays a link.
        Ok(target) => write_atomic(&target, text.as_bytes()),
        // No file yet (or a link to one): writing creates it.
        Err(_) => {
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            fs::write(path, text)
        }
    }
}

/// `text` with the `Theme=` line of its [Revmo] section set to `theme`:
/// replaced in place, or added after the section's header (and the
/// section added at the end if there is none). Nothing else changes.
fn with_theme(text: &str, theme: &str) -> String {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let content = |line: &str| line.trim().trim_start_matches('\u{feff}').to_string();
    let is_header = |line: &str| {
        let line = content(line);
        line.starts_with('[') && line.ends_with(']')
    };
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let setting = format!("Theme={}", escape(theme, None));

    let Some(header) = lines
        .iter()
        .position(|l| content(l) == format!("[{GROUP}]"))
    else {
        let mut out = text.to_string();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(eol);
        }
        if !out.trim().is_empty() {
            out.push_str(eol);
        }
        out.push_str(&format!("[{GROUP}]{eol}{setting}{eol}"));
        return out;
    };
    let section_end = lines[header + 1..]
        .iter()
        .position(|l| is_header(l))
        .map_or(lines.len(), |i| header + 1 + i);
    let existing = (header + 1..section_end).find(|&i| {
        content(lines[i])
            .split_once('=')
            .is_some_and(|(key, _)| key.trim() == "Theme")
    });

    let mut out = String::with_capacity(text.len() + setting.len() + 2);
    for (i, line) in lines.iter().enumerate() {
        if Some(i) == existing {
            // Keep the line's own ending, or lack of one at the end.
            let ending = &line[line.trim_end_matches(['\r', '\n']).len()..];
            out.push_str(&setting);
            out.push_str(ending);
            continue;
        }
        out.push_str(line);
        if i == header && existing.is_none() {
            if !line.ends_with('\n') {
                out.push_str(eol);
            }
            out.push_str(&setting);
            out.push_str(eol);
        }
    }
    out
}

fn load_from(path: &Path) -> Loaded {
    match fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => parse(&text),
            // Use what can be read, but never write over the rest.
            Err(e) => {
                let mut loaded = parse(&String::from_utf8_lossy(e.as_bytes()));
                let reason = "the file isn't valid UTF-8 text".to_string();
                loaded
                    .problems
                    .insert(0, format!("{reason}; parts may be misread"));
                loaded.unsaveable = Some(reason);
                loaded
            }
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => Loaded::default(),
        Err(e) => Loaded {
            problems: vec![format!("can't read {}: {e}", path.display())],
            unsaveable: Some("it can't be read".into()),
            ..Loaded::default()
        },
    }
}

/// The settings in `text`, each falling back to its default if missing or
/// invalid.
fn parse(text: &str) -> Loaded {
    // Some editors start files with a byte order mark.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut problems = check_lines(text);
    let file = KeyFile::parse(text);
    let Some(group) = file.group(GROUP) else {
        let has_settings = text.lines().any(|l| {
            let l = l.trim();
            !l.is_empty() && !l.starts_with('#')
        });
        let mut loaded = Loaded {
            problems,
            ..Loaded::default()
        };
        if has_settings {
            let reason = format!("there's no [{GROUP}] section");
            loaded
                .problems
                .push(format!("{reason}, so no setting applies"));
            loaded.unsaveable = Some(reason);
        }
        return loaded;
    };

    let mut config = Config::default();
    if let Some(theme) = group.string("Theme").filter(|t| !t.trim().is_empty()) {
        config.theme = theme.trim().to_string();
    }
    config.font = group.string("Font").unwrap_or_default().trim().to_string();
    if let Some(value) = group.string("Scale") {
        let (scale, problem) = parse_scale(&value);
        config.scale = scale;
        problems.extend(problem);
    }
    config.hidden = group
        .list("Hidden", ';')
        .iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();
    Loaded {
        config,
        problems,
        unsaveable: None,
    }
}

/// Lines revmo will ignore: unknown or repeated settings, settings outside
/// the [Revmo] section, other sections and lines that aren't settings.
fn check_lines(text: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let mut section: Option<&str> = None;
    let mut sections_seen = 0;
    let mut keys_seen: Vec<&str> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            if name == GROUP {
                sections_seen += 1;
                if sections_seen > 1 {
                    problems.push(format!("line {n}: a second [{GROUP}] section is ignored"));
                }
            } else if name.trim().eq_ignore_ascii_case(GROUP) {
                problems.push(format!("line {n}: [{name}] should be written [{GROUP}]"));
            } else {
                problems.push(format!("line {n}: unknown section [{}]", short(name)));
            }
            section = Some(name);
            continue;
        }
        let Some((key, _)) = line.split_once('=') else {
            problems.push(format!(
                "line {n}: \"{}\" isn't a Key=value setting",
                short(line)
            ));
            continue;
        };
        let key = key.trim();
        match section {
            None => problems.push(format!(
                "line {n}: {} comes before the [{GROUP}] section",
                short(key)
            )),
            Some(GROUP) if sections_seen == 1 => {
                if !KEYS.contains(&key) {
                    let hint = KEYS
                        .iter()
                        .find(|k| k.eq_ignore_ascii_case(key))
                        .map(|k| format!("; did you mean {k}?"))
                        .unwrap_or_default();
                    problems.push(format!("line {n}: unknown setting {}{hint}", short(key)));
                } else if keys_seen.contains(&key) {
                    problems.push(format!(
                        "line {n}: {key} is set again; the first one is used"
                    ));
                } else {
                    keys_seen.push(key);
                }
            }
            // Already reported with the section.
            Some(_) => {}
        }
    }
    problems
}

/// At most 40 characters of `text`, for messages.
fn short(text: &str) -> String {
    match text.char_indices().nth(40) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_string(),
    }
}

/// The scale `value` asks for (`None`: automatic), and what was wrong with
/// it, if anything.
fn parse_scale(value: &str) -> (Option<f32>, Option<String>) {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("auto") {
        return (None, None);
    }
    match value.parse::<f32>() {
        Ok(scale) if scale.is_finite() && scale > 0.0 => {
            let clamped = scale.clamp(MIN_SCALE, MAX_SCALE);
            let problem = (clamped != scale).then(|| {
                format!("Scale={value} is outside {MIN_SCALE}–{MAX_SCALE}; using {clamped}")
            });
            (Some(clamped), problem)
        }
        _ => (
            None,
            Some(format!(
                "Scale={} isn't a positive number like 1.5; using auto",
                short(value)
            )),
        ),
    }
}

const MIN_SCALE: f32 = 0.5;
const MAX_SCALE: f32 = 4.0;

#[cfg(test)]
mod tests {
    use super::*;

    fn config(text: &str) -> Config {
        parse(text).config
    }

    fn problems(text: &str) -> Vec<String> {
        parse(text).problems
    }

    #[test]
    fn reads_every_setting() {
        let loaded = parse(
            "[Revmo]\nTheme=Dracula\nFont=IBM Plex Mono\nScale=1.5\nHidden=a.desktop;b.desktop/new\\;window;\n",
        );
        assert_eq!(
            loaded.config,
            Config {
                theme: "Dracula".into(),
                font: "IBM Plex Mono".into(),
                scale: Some(1.5),
                hidden: vec!["a.desktop".into(), "b.desktop/new;window".into()],
            }
        );
        assert!(loaded.problems.is_empty());
    }

    #[test]
    fn saving_the_theme_changes_only_its_line() {
        let text =
            "# mine\n[Revmo]\n# dark one\nTheme=Nord\nFont=Iosevka\nOops=1\n\n[Other]\nTheme=x\n";
        assert_eq!(
            with_theme(text, "Dracula"),
            "# mine\n[Revmo]\n# dark one\nTheme=Dracula\nFont=Iosevka\nOops=1\n\n[Other]\nTheme=x\n"
        );
        // No Theme= yet: added right under the header.
        assert_eq!(
            with_theme("[Revmo]\nFont=Iosevka", "Nord"),
            "[Revmo]\nTheme=Nord\nFont=Iosevka"
        );
        // No section or no file: one is added.
        assert_eq!(with_theme("", "Nord"), "[Revmo]\nTheme=Nord\n");
        assert_eq!(
            with_theme("# notes", "Nord"),
            "# notes\n\n[Revmo]\nTheme=Nord\n"
        );
        // CRLF files stay CRLF, and the last line keeps having no newline.
        assert_eq!(
            with_theme("[Revmo]\r\nTheme=Nord\r\nFont=x\r\n", "Ferra"),
            "[Revmo]\r\nTheme=Ferra\r\nFont=x\r\n"
        );
        assert_eq!(
            with_theme("[Revmo]\nTheme=Nord", "Ferra"),
            "[Revmo]\nTheme=Ferra"
        );
        // What's saved reads back.
        assert_eq!(
            config(&with_theme(text, "Tokyo Night")).theme,
            "Tokyo Night"
        );
    }

    #[test]
    fn saves_through_symlinks_and_keeps_later_edits() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("dotfiles/revmo");
        fs::create_dir_all(real.parent().unwrap()).unwrap();
        fs::write(&real, "[Revmo]\nTheme=Nord\n").unwrap();
        let link = tmp.path().join("config");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        // An edit made after revmo started is kept.
        fs::write(&real, "[Revmo]\nTheme=Nord\nFont=Iosevka\n").unwrap();
        save_theme_to(&link, "Ferra").unwrap();
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_to_string(&real).unwrap(),
            "[Revmo]\nTheme=Ferra\nFont=Iosevka\n"
        );
    }

    #[test]
    fn missing_settings_use_defaults() {
        assert_eq!(parse(""), Loaded::default());
        assert_eq!(parse("[Revmo]\n"), Loaded::default());
        assert_eq!(parse("# just a comment\n"), Loaded::default());
        assert_eq!(config("[Revmo]\nTheme=\n").theme, theme::DEFAULT);
    }

    #[test]
    fn each_setting_overrides_its_default() {
        let c = config("[Revmo]\nFont= Iosevka \n");
        assert_eq!(c.font, "Iosevka");
        assert_eq!((c.theme.as_str(), c.scale), (theme::DEFAULT, None));
        // A bad value only affects its own setting.
        let loaded = parse("[Revmo]\nScale=big\nTheme=Nord\n");
        assert_eq!(
            (loaded.config.theme.as_str(), loaded.config.scale),
            ("Nord", None)
        );
        assert_eq!(loaded.problems.len(), 1);
    }

    #[test]
    fn parses_scale() {
        let scale = |s: &str| parse_scale(s).0;
        assert_eq!(scale("1.25"), Some(1.25));
        assert_eq!(scale(" 2 "), Some(2.0));
        assert_eq!(scale("10"), Some(MAX_SCALE));
        assert_eq!(scale("0.1"), Some(MIN_SCALE));
        for auto in ["", "auto", "AUTO"] {
            assert_eq!(parse_scale(auto), (None, None), "{auto}");
        }
        for bad in ["-1", "0", "NaN", "inf", "big", "1,5", "150%"] {
            let (scale, problem) = parse_scale(bad);
            assert_eq!(scale, None, "{bad}");
            assert!(problem.is_some(), "{bad}");
        }
        assert!(parse_scale("10").1.unwrap().contains("using 4"));
    }

    #[test]
    fn reports_lines_it_ignores() {
        let text = "Font=x\n[Revmo]\nThem=Nord\nscale=2\nTheme=A\nTheme=B\njunk\n[Other]\nKey=1\n";
        assert_eq!(
            problems(text),
            [
                "line 1: Font comes before the [Revmo] section",
                "line 3: unknown setting Them",
                "line 4: unknown setting scale; did you mean Scale?",
                "line 6: Theme is set again; the first one is used",
                "line 7: \"junk\" isn't a Key=value setting",
                "line 8: unknown section [Other]",
            ]
        );
        assert_eq!(config(text).theme, "A");
        assert_eq!(
            problems("[Revmo]\n[Revmo]\nTheme=X\n"),
            ["line 2: a second [Revmo] section is ignored"]
        );
    }

    #[test]
    fn a_file_without_the_section_is_not_saved_over() {
        let loaded = parse("[revmo]\nTheme=Nord\nHidden=a.desktop;\n");
        assert_eq!(loaded.config, Config::default());
        assert_eq!(
            loaded.problems,
            [
                "line 1: [revmo] should be written [Revmo]",
                "there's no [Revmo] section, so no setting applies",
            ]
        );
        assert!(loaded.unsaveable.is_some());
    }

    #[test]
    fn tolerates_bom_crlf_and_spaced_hidden_ids() {
        let loaded = parse("\u{feff}[Revmo]\r\nTheme=Nord\r\nHidden= a.desktop ; b.desktop;;\r\n");
        assert_eq!(loaded.problems, Vec::<String>::new());
        assert_eq!(loaded.config.theme, "Nord");
        assert_eq!(loaded.config.hidden, ["a.desktop", "b.desktop"]);
    }

    #[test]
    fn unreadable_files_are_reported_and_not_saved_over() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config");
        // Missing: defaults, nothing created.
        assert_eq!(load_from(&path), Loaded::default());
        assert!(!path.exists());

        fs::write(&path, b"[Revmo]\nTheme=Nord\nFont=Caf\xe9\n").unwrap();
        let loaded = load_from(&path);
        assert_eq!(loaded.config.theme, "Nord");
        assert!(loaded.unsaveable.is_some());
        assert!(loaded.problems[0].contains("UTF-8"));

        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        let loaded = load_from(&path);
        assert_eq!(loaded.config, Config::default());
        assert!(loaded.problems[0].starts_with("can't read"));
        assert!(loaded.unsaveable.is_some());
    }

    #[test]
    fn theme_names_survive_saving() {
        for name in ["Revmo Dark", "Tokyo Night", "Catppuccin Frappé"] {
            assert_eq!(config(&with_theme("", name)).theme, name);
        }
    }

    #[test]
    fn refuses_to_save_over_what_it_cannot_read() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config");
        for text in [&b"[revmo]\nTheme=Nord\n"[..], b"[Revmo]\nFont=Caf\xe9\n"] {
            fs::write(&path, text).unwrap();
            assert!(save_theme_to(&path, "Ferra").is_err());
            assert_eq!(fs::read(&path).unwrap(), text);
        }
        assert_eq!(
            save_theme_to(&path, "Ferra").unwrap_err().to_string(),
            "the file isn't valid UTF-8 text"
        );
    }
}
