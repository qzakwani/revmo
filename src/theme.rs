//! Color themes: two built in, the ones bundled in [`crate::bundled_themes`],
//! and `.theme` files the user puts in `~/.config/revmo/themes`:
//!
//! ```text
//! [Theme]
//! Name=Dracula
//! Background=#282a36
//! Text=#f8f8f2
//! Accent=#bd93f9
//! ```
//!
//! Background, Text and Accent are required; every other color (see
//! [`KEYS`]) is optional and worked out from those three when missing.

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use iced::theme::Palette;
use iced::theme::palette::is_dark;
use iced::{Color, Theme};
use rkyv::{Archive, Deserialize, Serialize};

use crate::bundled_themes::BUNDLED;
use crate::{HOME_DIR, store};
use revmo::xdg::base_dirs::config_home;
use revmo::xdg::keyfile::KeyFile;

pub const DEFAULT: &str = "Revmo Dark";
const GROUP: &str = "Theme";
const EXTENSION: &str = "theme";

/// Every color the UI draws with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    pub background: Color,
    /// A step off the background: the footer and input fields.
    pub surface: Color,
    /// A further step: the highlighted row while the actions pane is open,
    /// with matched letters drawn on it.
    pub surface_strong: Color,
    /// The window's outer border.
    pub frame: Color,
    /// Dividers between sections.
    pub border: Color,
    pub border_strong: Color,
    pub text: Color,
    pub text_dim: Color,
    pub text_muted: Color,
    pub text_faint: Color,
    /// Text on the background (the prompt, key names), the focused field's
    /// outline, the run button and the actions pane's marker bar.
    pub accent: Color,
    pub on_accent: Color,
    /// The selected row, and selected text in text fields when the text
    /// reads on it.
    pub selection: Color,
    pub on_selection: Color,
    pub danger: Color,
    pub success: Color,
    /// Matched letters in results.
    pub highlight: Color,
}

/// The file keys, in the order of [`Colors::to_array`].
pub const KEYS: [&str; 17] = [
    "Background",
    "Surface",
    "SurfaceStrong",
    "Frame",
    "Border",
    "BorderStrong",
    "Text",
    "TextDim",
    "TextMuted",
    "TextFaint",
    "Accent",
    "OnAccent",
    "Selection",
    "OnSelection",
    "Danger",
    "Success",
    "Highlight",
];

impl Colors {
    fn to_array(self) -> [Color; 17] {
        [
            self.background,
            self.surface,
            self.surface_strong,
            self.frame,
            self.border,
            self.border_strong,
            self.text,
            self.text_dim,
            self.text_muted,
            self.text_faint,
            self.accent,
            self.on_accent,
            self.selection,
            self.on_selection,
            self.danger,
            self.success,
            self.highlight,
        ]
    }

    fn from_array(c: [Color; 17]) -> Self {
        Colors {
            background: c[0],
            surface: c[1],
            surface_strong: c[2],
            frame: c[3],
            border: c[4],
            border_strong: c[5],
            text: c[6],
            text_dim: c[7],
            text_muted: c[8],
            text_faint: c[9],
            accent: c[10],
            on_accent: c[11],
            selection: c[12],
            on_selection: c[13],
            danger: c[14],
            success: c[15],
            highlight: c[16],
        }
    }

    /// The full set from the three required colors, with any of the others
    /// that `given` provides (looked up by file key).
    fn derive(
        background: Color,
        text: Color,
        accent: Color,
        given: impl Fn(&str) -> Option<Color>,
    ) -> Self {
        let get = |key: &str, default: Color| given(key).unwrap_or(default);
        let shade = |factor: f32| blend(background, text, factor);
        let dark = is_dark(background);
        let border = get("Border", shade(0.14));
        let selection = get("Selection", accent);
        Colors {
            background,
            surface: get("Surface", shade(0.03)),
            surface_strong: get("SurfaceStrong", shade(0.07)),
            // Dividers left out still leave the outline.
            frame: get("Frame", if border.a == 0.0 { shade(0.14) } else { border }),
            border,
            border_strong: get("BorderStrong", shade(0.21)),
            text,
            text_dim: get("TextDim", shade(0.7)),
            text_muted: get("TextMuted", shade(0.52)),
            text_faint: get("TextFaint", shade(0.36)),
            accent,
            on_accent: get("OnAccent", readable_on(accent, background, text)),
            selection,
            on_selection: get("OnSelection", readable_on(selection, background, text)),
            danger: get("Danger", if dark { rgb(0xe07b6a) } else { rgb(0xb8463a) }),
            success: get("Success", if dark { rgb(0x8fbf6a) } else { rgb(0x4d7c2f) }),
            highlight: get("Highlight", accent),
        }
    }
}

impl Colors {
    /// Behind selected text in text fields: the selection when the text
    /// reads on it, else a tint of it.
    pub fn text_selection(self) -> Color {
        if self.text.relative_contrast(self.selection) >= 4.5 {
            self.selection
        } else {
            Color {
                a: 0.35,
                ..self.selection
            }
        }
    }
}

/// A named set of colors, as cached.
#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ThemeDef {
    pub name: String,
    /// [`Colors`] as RGBA, in [`KEYS`] order.
    rgba: [u32; 17],
}

impl ThemeDef {
    pub fn new(name: String, colors: Colors) -> Self {
        ThemeDef {
            name,
            rgba: colors
                .to_array()
                .map(|c| u32::from_be_bytes(c.into_rgba8())),
        }
    }

    pub fn colors(&self) -> Colors {
        Colors::from_array(self.rgba.map(|c| {
            let [r, g, b, a] = c.to_be_bytes();
            Color::from_rgba8(r, g, b, f32::from(a) / 255.0)
        }))
    }

    /// The iced theme with the same colors, for widgets' default styles.
    pub fn to_iced(&self) -> Theme {
        let c = self.colors();
        Theme::custom(
            self.name.clone(),
            Palette {
                background: c.background,
                text: c.text,
                primary: c.accent,
                success: c.success,
                warning: c.accent,
                danger: c.danger,
            },
        )
    }
}

/// The themes that need no files.
pub fn built_in() -> [ThemeDef; 2] {
    [
        ThemeDef::new(
            DEFAULT.into(),
            Colors {
                background: rgb(0x0f1012),
                surface: rgb(0x141518),
                surface_strong: rgb(0x1e2024),
                frame: rgb(0x2c2e33),
                border: rgb(0x2c2e33),
                border_strong: rgb(0x3a3c42),
                text: rgb(0xd9d9dc),
                text_dim: rgb(0x9a9a9e),
                text_muted: rgb(0x6a6b70),
                text_faint: rgb(0x4a4c52),
                accent: rgb(0xf8a052),
                on_accent: rgb(0x1b1206),
                selection: rgb(0xf8a052),
                on_selection: rgb(0x1b1206),
                danger: rgb(0xe07b6a),
                success: rgb(0x8fbf6a),
                highlight: rgb(0xf8a052),
            },
        ),
        ThemeDef::new(
            "Revmo Light".into(),
            Colors {
                background: rgb(0xf7f6f2),
                surface: rgb(0xedebe5),
                surface_strong: rgb(0xe6e3db),
                frame: rgb(0xc9c6bd),
                border: rgb(0xe2e0d8),
                border_strong: rgb(0xc9c6bd),
                text: rgb(0x1b1b1d),
                text_dim: rgb(0x48484c),
                text_muted: rgb(0x6e6e72),
                text_faint: rgb(0x9a9a9e),
                accent: rgb(0xab5900),
                on_accent: rgb(0xf7f6f2),
                selection: rgb(0xf8a052),
                on_selection: rgb(0x1b1206),
                danger: rgb(0xb4443a),
                success: rgb(0x4d7c2f),
                highlight: rgb(0x9c5100),
            },
        ),
    ]
}

/// The themes in [`BUNDLED`], with their missing colors worked out.
pub fn bundled() -> Vec<ThemeDef> {
    BUNDLED
        .iter()
        .map(|b| {
            let given = |key: &str| {
                b.other
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|&(_, c)| rgb(c))
            };
            let colors = Colors::derive(rgb(b.background), rgb(b.text), rgb(b.accent), given);
            ThemeDef::new(b.name.to_string(), colors)
        })
        .collect()
}

/// The built-in themes, then the bundled ones and `files` sorted by name.
/// A file theme replaces a bundled one with the same name, but not a
/// built-in one.
pub fn with_built_in(files: Vec<ThemeDef>) -> Vec<ThemeDef> {
    let mut all = built_in().to_vec();
    let mut rest: Vec<ThemeDef> = files
        .into_iter()
        .filter(|f| !all.iter().any(|b| b.name == f.name))
        .collect();
    let bundled: Vec<ThemeDef> = bundled()
        .into_iter()
        .filter(|b| !rest.iter().any(|f| f.name == b.name))
        .collect();
    rest.extend(bundled);
    rest.sort_by_cached_key(|t| t.name.to_lowercase());
    all.extend(rest);
    all
}

/// The theme called `name`, if there is one.
pub fn by_name<'a>(themes: &'a [ThemeDef], name: &str) -> Option<&'a ThemeDef> {
    themes.iter().find(|t| t.name == name)
}

fn position(themes: &[ThemeDef], current: &str) -> usize {
    themes.iter().position(|t| t.name == current).unwrap_or(0)
}

pub fn next<'a>(themes: &'a [ThemeDef], current: &str) -> &'a ThemeDef {
    &themes[(position(themes, current) + 1) % themes.len()]
}

pub fn previous<'a>(themes: &'a [ThemeDef], current: &str) -> &'a ThemeDef {
    &themes[(position(themes, current) + themes.len() - 1) % themes.len()]
}

/// The theme files' themes from the last scan and the fingerprint they were
/// scanned with, without checking that they're still current.
pub fn cached() -> Option<(u64, Vec<ThemeDef>)> {
    store::get_themes()
}

/// A fresh scan if the theme files changed since `fingerprint` (or there is
/// none), else `None`. Blocks on file IO.
pub fn scan_if_changed(fingerprint: Option<u64>) -> Option<Vec<ThemeDef>> {
    let dirs = dirs(Path::new(HOME_DIR.as_str()));
    let current = self::fingerprint(&dirs);
    (fingerprint != Some(current)).then(|| scan_and_store(&dirs, current))
}

/// A fresh scan, whether or not anything changed. Blocks on file IO.
pub fn rescan() -> Vec<ThemeDef> {
    let dirs = dirs(Path::new(HOME_DIR.as_str()));
    let current = fingerprint(&dirs);
    scan_and_store(&dirs, current)
}

/// Scans with the fingerprint taken before scanning, so a file changed
/// mid-scan triggers another scan next time.
fn scan_and_store(dirs: &[PathBuf], fingerprint: u64) -> Vec<ThemeDef> {
    let themes = scan(dirs);
    store::store_themes(fingerprint, &themes);
    themes
}

/// Folders holding `.theme` files: the user's own, in `$XDG_CONFIG_HOME`.
pub fn dirs(home: &Path) -> Vec<PathBuf> {
    vec![config_home(home).join("revmo/themes")]
}

/// The `.theme` files directly inside `dir`, sorted by name.
fn files_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == EXTENSION))
        .collect();
    files.sort();
    files
}

/// Changes whenever a theme file is added, removed or edited.
pub fn fingerprint(dirs: &[PathBuf]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for dir in dirs {
        dir.hash(&mut hasher);
        for file in files_in(dir) {
            file.hash(&mut hasher);
            fs::metadata(&file)
                .and_then(|m| m.modified())
                .ok()
                .hash(&mut hasher);
        }
    }
    hasher.finish()
}

/// Reads every theme file in `dirs`, sorted by name. When two files share a
/// name, the one in the earlier folder wins. Blocks on file IO.
pub fn scan(dirs: &[PathBuf]) -> Vec<ThemeDef> {
    let mut themes: Vec<ThemeDef> = Vec::new();
    for path in dirs.iter().flat_map(|d| files_in(d)) {
        let Ok(contents) = fs::read_to_string(&path) else {
            eprintln!("Error: can't read {}", path.display());
            continue;
        };
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        match parse(&contents, &stem) {
            Ok(theme) if !themes.iter().any(|t| t.name == theme.name) => themes.push(theme),
            Ok(_) => {}
            Err(e) => eprintln!("Error: skipping theme {}: {e}", path.display()),
        }
    }
    themes.sort_by_cached_key(|t| t.name.to_lowercase());
    themes
}

/// A theme file's contents; `fallback_name` is used when it has no `Name`.
pub fn parse(contents: &str, fallback_name: &str) -> Result<ThemeDef, String> {
    let file = KeyFile::parse(contents);
    let group = file
        .group(GROUP)
        .ok_or_else(|| format!("no [{GROUP}] group"))?;
    for key in KEYS {
        let Some(value) = group.raw(key) else {
            continue;
        };
        if is_none(value) && key == "Background" {
            return Err(
                "Background can't be none; your compositor makes windows see-through".into(),
            );
        }
        if !is_none(value) && parse_color(value).is_none() {
            return Err(format!("{key}={value} is not a #rrggbb color or none"));
        }
    }
    let given = |key: &str| {
        let value = group.raw(key)?;
        match is_none(value) {
            true => Some(Color::TRANSPARENT),
            false => parse_color(value),
        }
    };
    let required = |key: &str| given(key).ok_or_else(|| format!("{key} is missing"));
    // The window is opaque, so a see-through background would only come out
    // darker; its alpha is ignored.
    let background = Color {
        a: 1.0,
        ..required("Background")?
    };
    let colors = Colors::derive(background, required("Text")?, required("Accent")?, given);
    // Trimmed, as the config's Theme= is, so names always match.
    let name = group
        .string("Name")
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| fallback_name.trim().to_string());
    Ok(ThemeDef::new(name, colors))
}

/// `none` or `transparent`: not drawn at all, so the background shows in
/// its place. Any color but the background can be.
fn is_none(value: &str) -> bool {
    let value = value.trim();
    value.eq_ignore_ascii_case("none") || value.eq_ignore_ascii_case("transparent")
}

/// `#rrggbb` or `#rrggbbaa`.
fn parse_color(value: &str) -> Option<Color> {
    let hex = value.trim().strip_prefix('#')?;
    if !matches!(hex.len(), 6 | 8) || !hex.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    let alpha = if hex.len() == 8 { byte(6)? } else { 255 };
    Some(Color::from_rgba8(
        byte(0)?,
        byte(2)?,
        byte(4)?,
        f32::from(alpha) / 255.0,
    ))
}

const fn rgb(hex: u32) -> Color {
    Color::from_rgb8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// Mixes in sRGB, so the factor matches how far between the two colors
/// the result looks.
fn blend(a: Color, b: Color, factor: f32) -> Color {
    let lerp = |x: f32, y: f32| x + (y - x) * factor;
    Color::from_rgb(lerp(a.r, b.r), lerp(a.g, b.g), lerp(a.b, b.b))
}

/// Text for `fill`: the theme's background or text color, whichever reads
/// better, or black or white if neither reaches 4.5:1.
fn readable_on(fill: Color, background: Color, text: Color) -> Color {
    let best = |a: Color, b: Color| {
        if fill.relative_contrast(a) >= fill.relative_contrast(b) {
            a
        } else {
            b
        }
    };
    let color = best(background, text);
    if fill.relative_contrast(color) >= 4.5 {
        color
    } else {
        best(Color::BLACK, Color::WHITE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn themes() -> Vec<ThemeDef> {
        with_built_in(vec![
            parse(
                "[Theme]\nName=Zz\nBackground=#000000\nText=#ffffff\nAccent=#ff0000",
                "zz",
            )
            .unwrap(),
        ])
    }

    #[test]
    fn cycles_by_name() {
        let themes = themes();
        // Built-ins first, then the rest by name: "Zz" comes last.
        assert_eq!(next(&themes, "Zz").name, DEFAULT);
        assert_eq!(previous(&themes, DEFAULT).name, "Zz");
        assert_eq!(next(&themes, "No Such Theme").name, "Revmo Light");
        assert_eq!(next(&themes, "Revmo Light").name, "Catppuccin Frappé");
        assert!(by_name(&themes, "Zz").is_some());
        assert!(by_name(&themes, "No Such Theme").is_none());
    }

    #[test]
    fn parses_and_derives() {
        let theme = parse(
            "[Theme]\nBackground=#0f1012\nText=#d9d9dc\nAccent=#f0a940\nDanger=#ff000080",
            "mine",
        )
        .unwrap();
        assert_eq!(theme.name, "mine");
        let c = theme.colors();
        assert_eq!(c.background, rgb(0x0f1012));
        assert_eq!(c.selection, c.accent);
        assert_eq!(c.danger.into_rgba8(), [255, 0, 0, 128]);
        // Derived shades sit between the background and the text.
        assert!(c.border.relative_contrast(c.background) > 1.0);
        assert!(
            c.text_muted.relative_contrast(c.background) < c.text.relative_contrast(c.background)
        );
    }

    #[test]
    fn none_leaves_a_fill_out() {
        let theme = parse(
            "[Theme]\nBackground=#000000\nText=#ffffff\nAccent=#ff0000\nBorder=None\nSurface=transparent\nSelection=none",
            "x",
        )
        .unwrap();
        let c = theme.colors();
        assert_eq!(c.border, Color::TRANSPARENT);
        assert_eq!(c.surface, Color::TRANSPARENT);
        assert_eq!(c.selection, Color::TRANSPARENT);
        // No dividers still leaves the outline.
        assert_eq!(c.frame.a, 1.0);
        assert_eq!(c.background, rgb(0x000000));
    }

    #[test]
    fn rejects_bad_files() {
        assert!(parse("[Theme]\nBackground=#000000\nText=#ffffff", "x").is_err());
        assert!(
            parse(
                "[Theme]\nBackground=#000\nText=#ffffff\nAccent=#ff0000",
                "x"
            )
            .is_err()
        );
        let bad_optional = "[Theme]\nBackground=#000000\nText=#ffffff\nAccent=#ff0000\nBorder=red";
        assert!(parse(bad_optional, "x").is_err());
        let no_background = "[Theme]\nBackground=none\nText=#ffffff\nAccent=#ff0000";
        assert!(parse(no_background, "x").is_err());
        assert!(parse("Background=#000000", "x").is_err());
    }

    #[test]
    fn first_folder_wins_and_built_ins_stay() {
        let tmp = tempfile::tempdir().unwrap();
        let (user, installed) = (tmp.path().join("user"), tmp.path().join("installed"));
        fs::create_dir_all(&user).unwrap();
        fs::create_dir_all(&installed).unwrap();
        let theme = |accent: &str| {
            format!("[Theme]\nName=X\nBackground=#000000\nText=#ffffff\nAccent={accent}")
        };
        fs::write(user.join("x.theme"), theme("#ff0000")).unwrap();
        fs::write(installed.join("x.theme"), theme("#00ff00")).unwrap();
        fs::write(
            installed.join("dark.theme"),
            "[Theme]\nName=Revmo Dark\nBackground=#000000\nText=#ffffff\nAccent=#00ff00",
        )
        .unwrap();
        fs::write(installed.join("broken.theme"), "garbage").unwrap();
        fs::write(installed.join("notes.txt"), theme("#0000ff")).unwrap();

        let dirs = [user, installed];
        let before = fingerprint(&dirs);
        let scanned = scan(&dirs);
        assert_eq!(scanned.len(), 2);
        let x = by_name(&scanned, "X").unwrap();
        assert_eq!(x.colors().accent, rgb(0xff0000));

        let all = with_built_in(scanned);
        assert_eq!(all.len(), 2 + BUNDLED.len() + 1);
        assert_eq!(by_name(&all, DEFAULT).unwrap(), &built_in()[0]);

        fs::remove_file(dirs[0].join("x.theme")).unwrap();
        assert_ne!(fingerprint(&dirs), before);
    }

    #[test]
    fn file_themes_replace_bundled_ones() {
        let mine = parse(
            "[Theme]\nName=Dracula\nBackground=#000000\nText=#ffffff\nAccent=#00ff00",
            "dracula",
        )
        .unwrap();
        let all = with_built_in(vec![mine.clone()]);
        assert_eq!(all.len(), 2 + BUNDLED.len());
        assert_eq!(by_name(&all, "Dracula"), Some(&mine));
    }

    /// Checks a shipped theme fails because its author's palette has no
    /// color that both passes and fits the role.
    const EXEMPT: [(&str, &str); 20] = [
        // No chromatic Latte color reaches 4.5 on a surface apart from Base.
        ("Catppuccin Latte", "highlight marked"),
        // Lotus has no red, orange or pink that passes.
        ("Kanagawa Lotus", "danger"),
        ("Kanagawa Lotus", "danger toast"),
        // Red is 4.46 on Mantle, the darkest step off Base.
        ("Catppuccin Latte", "danger toast"),
        // Yellow is 4.05 on base02; only green, cyan and the grays pass.
        ("Solarized Dark", "danger toast"),
        // Solarized Light has no red, orange or magenta that passes.
        ("Solarized Light", "danger"),
        ("Solarized Light", "danger toast"),
        // The body text itself is low contrast, so any visible surface
        // behind it takes it under 4.5.
        ("Solarized Dark", "text surface"),
        ("Solarized Dark", "text marked"),
        ("Solarized Dark", "search selection"),
        ("Solarized Dark", "field selection"),
        ("Solarized Dark", "text muted surface"),
        ("Solarized Light", "text"),
        ("Solarized Light", "text dim"),
        ("Solarized Light", "text muted"),
        ("Solarized Light", "text muted surface"),
        ("Solarized Light", "text surface"),
        ("Solarized Light", "text marked"),
        ("Solarized Light", "search selection"),
        ("Solarized Light", "field selection"),
    ];

    #[test]
    fn shipped_themes_are_readable() {
        // Bundled themes take every color from their palette, none derived.
        let required = ["Background", "Text", "Accent"];
        for b in &BUNDLED {
            for (key, _) in b.other {
                assert!(
                    KEYS.contains(key) && !required.contains(key),
                    "{}: {key} isn't an optional color",
                    b.name
                );
            }
            for key in KEYS.iter().filter(|k| !required.contains(k)) {
                let count = b.other.iter().filter(|(k, _)| k == key).count();
                assert_eq!(count, 1, "{}: {key} set {count} times", b.name);
            }
        }
        let mut names: Vec<String> = Vec::new();
        let mut failures: Vec<String> = Vec::new();
        let mut exempted = EXEMPT.to_vec();
        for theme in built_in().into_iter().chain(bundled()) {
            assert!(!names.contains(&theme.name), "{} twice", theme.name);
            names.push(theme.name.clone());
            let c = theme.colors();
            let secondary = blend(c.selection, c.on_selection, 0.75);
            let tint = c.text_selection();
            let over = |base: Color| blend(base, Color { a: 1.0, ..tint }, tint.a);
            // WCAG AA: 4.5 for text, 3 for secondary text and the marked
            // row's bar. Faint text need only stay visible, and the two rows
            // need only stand apart.
            for (check, a, b, min) in [
                ("text", c.text, c.background, 4.5),
                ("text surface", c.text, c.surface, 4.5),
                ("text marked", c.text, c.surface_strong, 4.5),
                ("text dim", c.text_dim, c.background, 4.5),
                ("text muted", c.text_muted, c.background, 3.0),
                ("text muted surface", c.text_muted, c.surface, 3.0),
                ("text faint", c.text_faint, c.background, 2.0),
                ("accent", c.accent, c.background, 4.5),
                ("mark bar", c.accent, c.surface_strong, 3.0),
                ("on accent", c.on_accent, c.accent, 4.5),
                ("highlight", c.highlight, c.background, 4.5),
                ("highlight marked", c.highlight, c.surface_strong, 4.5),
                ("danger", c.danger, c.background, 4.5),
                ("danger toast", c.danger, c.surface, 4.5),
                ("on selection", c.on_selection, c.selection, 4.5),
                ("selection secondary", secondary, c.selection, 3.0),
                ("search selection", c.text, over(c.background), 4.5),
                ("field selection", c.text, over(c.surface), 4.5),
                ("selection row", c.selection, c.background, 1.3),
                ("selection vs marked", c.selection, c.surface_strong, 1.15),
            ] {
                let contrast = a.relative_contrast(b);
                let entry = (theme.name.as_str(), check);
                let exempt = EXEMPT.contains(&entry);
                exempted.retain(|e| *e != entry);
                if contrast < min && !exempt {
                    failures.push(format!("{}: {check} {contrast:.2}", theme.name));
                }
                if contrast >= min && exempt {
                    failures.push(format!("{}: {check} passes, unexempt it", theme.name));
                }
            }
            // Each step of text fainter than the last, or the same.
            let steps = [c.text, c.text_dim, c.text_muted, c.text_faint]
                .map(|t| t.relative_contrast(c.background));
            if steps.windows(2).any(|w| w[1] > w[0]) {
                failures.push(format!("{}: text steps out of order {steps:?}", theme.name));
            }
        }
        assert!(exempted.is_empty(), "no such checks: {exempted:?}");
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
