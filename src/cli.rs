//! The command line. `revmo` alone opens the launcher, `menu` and
//! `clipboard` open it in a mode, and the list commands print what the
//! config file's settings take and exit without opening a window.

use std::io::{self, Write};

use crate::modes::columns::Columns;
use crate::parser::{self, App};
use crate::{check_font, config, styles};

pub const USAGE: &str = "\
Usage: revmo [COMMAND]

With no command, opens the launcher (or closes it if it's open).

Commands:
  clipboard          Open in clipboard mode, like typing \";\" in the launcher;
                     needs ClipboardList= and ClipboardPick= in the config
  menu [OPTIONS]     Pick one of the lines read from stdin and print it, like
                     dmenu; exits 1 if closed without picking
  list-apps          List apps and their actions, with the IDs Hidden= takes
  list-fonts         List installed font families, as Font= takes them

Menu options:
  -p PROMPT          Show PROMPT before the input
  --with-nth COLS    Show and search only these columns of each line, like
                     fzf: 2, 1,3 or 2.. (2 to the end); the whole line is
                     still printed
  --delimiter SEP    What separates the columns (default: a tab, \\t)

In menu:
  enter              Print the selected line (or what's typed, if nothing
                     matches)
  shift+enter        Print what's typed
  ctrl+enter         Print the selected line and keep picking
  tab                Copy the selected line into the input

Options:
  -h, --help         Show this help
  -V, --version      Show the version
";

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Open,
    Clipboard,
    /// `revmo menu`, with the prompt shown before the input and the
    /// columns of the lines to show.
    Menu {
        prompt: Option<String>,
        columns: Option<Columns>,
    },
    ListApps,
    ListFonts,
    Help,
    Version,
}

/// The command `args` (without the program name) ask for.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Ok(Command::Open);
    };
    let command = match first.as_str() {
        "menu" => return parse_menu(args),
        "clipboard" => Command::Clipboard,
        "list-apps" => Command::ListApps,
        "list-fonts" => Command::ListFonts,
        "-h" | "--help" => Command::Help,
        "-V" | "--version" => Command::Version,
        other => return Err(format!("unknown command \"{other}\"")),
    };
    match args.next() {
        Some(extra) => Err(format!("{first} takes no arguments, got \"{extra}\"")),
        None => Ok(command),
    }
}

/// `menu`'s options, from the arguments after it. The long ones take
/// their value as the next argument or after `=`, as in fzf.
fn parse_menu(mut args: impl Iterator<Item = String>) -> Result<Command, String> {
    let (mut prompt, mut with_nth, mut delimiter) = (None, None, None);
    while let Some(arg) = args.next() {
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value.to_string())),
            _ => (arg.as_str(), None),
        };
        let slot = match name {
            "-p" if inline.is_none() => &mut prompt,
            "--with-nth" => &mut with_nth,
            "--delimiter" => &mut delimiter,
            _ => return Err(format!("menu: unknown option \"{arg}\"")),
        };
        match inline.or_else(|| args.next()) {
            Some(value) => *slot = Some(value),
            None => return Err(format!("menu: {name} needs a value after it")),
        }
    }
    // Written as in fzf: `--delimiter '\t'` is a tab.
    let delimiter = delimiter.map(|d| d.replace("\\t", "\t"));
    let columns = match with_nth {
        Some(spec) => Some(
            Columns::parse(&spec, delimiter.as_deref())
                .map_err(|e| format!("menu: --with-nth: {e}"))?,
        ),
        None => None,
    };
    Ok(Command::Menu { prompt, columns })
}

impl Command {
    /// Whether this opens revmo's window rather than printing something.
    pub fn opens_window(&self) -> bool {
        matches!(
            self,
            Command::Open | Command::Clipboard | Command::Menu { .. }
        )
    }
}

/// Prints what `command` asks for.
pub fn run(command: Command) {
    let out = match command {
        Command::Open | Command::Clipboard | Command::Menu { .. } => return,
        Command::Help => USAGE.to_string(),
        Command::Version => format!("revmo {}\n", env!("CARGO_PKG_VERSION")),
        Command::ListApps => {
            let hidden = config::Config::load().config.hidden;
            apps_table(&current_apps(), &hidden)
        }
        Command::ListFonts => {
            let (in_use, problem) = check_font(&config::Config::load().config.font);
            if let Some(problem) = problem {
                eprintln!("revmo: {problem}");
            }
            fonts_table(&styles::installed_families(), &in_use)
        }
    };
    match io::stdout().lock().write_all(out.as_bytes()) {
        // Piped into something like `head` that stopped reading: what it
        // read is all that was wanted.
        Err(e) if e.kind() != io::ErrorKind::BrokenPipe => {
            eprintln!("revmo: can't print: {e}");
            std::process::exit(1);
        }
        _ => {}
    }
}

/// How many terminal columns `text` takes: two for wide characters (CJK,
/// fullwidth forms, most emoji), one for the rest.
fn display_width(text: &str) -> usize {
    text.chars()
        .map(|c| match c as u32 {
            0x1100..=0x115F
            | 0x2E80..=0x303E
            | 0x3041..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA000..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1F64F
            | 0x1F900..=0x1F9FF
            | 0x20000..=0x3FFFD => 2,
            _ => 1,
        })
        .sum()
}

/// `text` followed by spaces up to `width` columns.
fn padded(text: &str, width: usize) -> String {
    let pad = width.saturating_sub(display_width(text));
    format!("{text}{}", " ".repeat(pad))
}

/// The apps the launcher would list, rescanned only if something changed.
fn current_apps() -> Vec<App> {
    let cached = App::cached();
    match App::scan_if_changed(cached.as_ref().map(|(fingerprint, _)| *fingerprint)) {
        Some(apps) => apps,
        None => cached.map(|(_, apps)| apps).unwrap_or_default(),
    }
}

/// One line per app, each followed by its actions (indented), with the ID
/// last so it's easy to cut out. Hidden ones are marked; the column only
/// appears if there are any.
fn apps_table(apps: &[App], hidden: &[String]) -> String {
    let parents = parser::parents(apps);
    let mut rows = Vec::new();
    for (i, app) in apps.iter().enumerate() {
        if parents[i] != i {
            continue;
        }
        let label = match &app.action {
            None => app.name.clone(),
            // An action whose app isn't in the list.
            Some(action) => format!("{}: {action}", app.name),
        };
        rows.push((label, app));
        for (j, action) in apps.iter().enumerate() {
            if j != i && parents[j] == i {
                let name = action.action.as_deref().unwrap_or(&action.name);
                rows.push((format!("  {name}"), action));
            }
        }
    }

    let width = rows
        .iter()
        .map(|(l, _)| display_width(l))
        .max()
        .unwrap_or(0);
    let any_hidden = apps.iter().any(|a| a.is_hidden(hidden));
    let mut out = String::new();
    for (label, app) in rows {
        let mark = match (any_hidden, app.is_hidden(hidden)) {
            (false, _) => "",
            (true, true) => "hidden  ",
            (true, false) => "        ",
        };
        out.push_str(&format!("{}  {mark}{}\n", padded(&label, width), app.key));
    }
    out
}

/// The bundled font, then every installed family, each tagged as
/// monospaced and/or in use.
fn fonts_table(families: &[(String, bool)], in_use: &str) -> String {
    let bundled = (styles::BUNDLED_FAMILY.to_string(), true);
    let rows: Vec<(&str, Vec<&str>)> = std::iter::once(&bundled)
        .chain(families)
        .enumerate()
        .map(|(i, (name, mono))| {
            let mut tags = Vec::new();
            if i == 0 {
                tags.push("bundled");
            }
            if *mono {
                tags.push("mono");
            }
            let current = match i {
                0 => in_use.is_empty(),
                _ => name.eq_ignore_ascii_case(in_use),
            };
            if current {
                tags.push("in use");
            }
            (name.as_str(), tags)
        })
        .collect();

    let width = rows
        .iter()
        .map(|(n, _)| display_width(n))
        .max()
        .unwrap_or(0);
    let mut out = String::new();
    for (name, tags) in rows {
        let line = format!("{}  {}", padded(name, width), tags.join(", "));
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use revmo::xdg::exec::{EntryInfo, Exec};
    use std::path::Path;

    fn args(list: &[&str]) -> Result<Command, String> {
        parse(list.iter().map(|a| a.to_string()))
    }

    fn app(key: &str, name: &str, action: Option<&str>) -> App {
        let info = EntryInfo {
            name,
            icon: None,
            path: Path::new("/x.desktop"),
        };
        App {
            key: key.into(),
            name: name.into(),
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
    fn parses_commands() {
        assert_eq!(args(&[]), Ok(Command::Open));
        assert_eq!(args(&["list-apps"]), Ok(Command::ListApps));
        assert_eq!(args(&["list-fonts"]), Ok(Command::ListFonts));
        assert_eq!(args(&["-h"]), Ok(Command::Help));
        assert_eq!(args(&["--help"]), Ok(Command::Help));
        assert_eq!(args(&["-V"]), Ok(Command::Version));
        assert_eq!(args(&["--version"]), Ok(Command::Version));
        assert_eq!(args(&["clipboard"]), Ok(Command::Clipboard));
        assert_eq!(args(&["list"]), Err("unknown command \"list\"".to_string()));
        assert_eq!(
            args(&["list-apps", "x"]),
            Err("list-apps takes no arguments, got \"x\"".to_string())
        );
    }

    #[test]
    fn parses_menu_options() {
        let menu = |prompt: Option<&str>, columns: Option<Columns>| {
            Ok(Command::Menu {
                prompt: prompt.map(Into::into),
                columns,
            })
        };
        assert_eq!(args(&["menu"]), menu(None, None));
        assert_eq!(args(&["menu", "-p", "run:"]), menu(Some("run:"), None));
        assert_eq!(
            args(&["menu", "-p"]),
            Err("menu: -p needs a value after it".to_string())
        );
        let cols = |spec, sep| Some(Columns::parse(spec, sep).unwrap());
        assert_eq!(
            args(&["menu", "--with-nth", "2.."]),
            menu(None, cols("2..", None))
        );
        assert_eq!(
            args(&["menu", "--with-nth=1,3", "--delimiter=: ", "-p", "x"]),
            menu(Some("x"), cols("1,3", Some(": ")))
        );
        // The order doesn't matter, and \t is a tab, as in fzf.
        assert_eq!(
            args(&["menu", "--delimiter", "\\t", "--with-nth", "2"]),
            menu(None, cols("2", None))
        );
        assert_eq!(
            args(&["menu", "--with-nth", "0"]),
            Err("menu: --with-nth: \"0\" isn't a list of columns like 2, 1,3 or 2..".to_string())
        );
        assert_eq!(
            args(&["menu", "-l", "10"]),
            Err("menu: unknown option \"-l\"".to_string())
        );
        assert_eq!(
            args(&["clipboard", "x"]),
            Err("clipboard takes no arguments, got \"x\"".to_string())
        );
    }

    #[test]
    fn lists_apps_with_their_actions() {
        let apps = [
            app("firefox.desktop", "Firefox", None),
            app(
                "firefox.desktop/private",
                "Firefox",
                Some("New Private Window"),
            ),
            app("org.gnome.Logs.desktop", "Logs", None),
            app("gone.desktop/new", "Gone", Some("New")),
        ];
        assert_eq!(
            apps_table(&apps, &[]),
            "\
Firefox               firefox.desktop
  New Private Window  firefox.desktop/private
Logs                  org.gnome.Logs.desktop
Gone: New             gone.desktop/new
"
        );
        // A hidden app's actions are hidden with it.
        assert_eq!(
            apps_table(&apps, &["firefox.desktop".to_string()]),
            "\
Firefox               hidden  firefox.desktop
  New Private Window  hidden  firefox.desktop/private
Logs                          org.gnome.Logs.desktop
Gone: New                     gone.desktop/new
"
        );
    }

    #[test]
    fn pads_wide_characters_by_their_width() {
        assert_eq!(display_width("Logs"), 4);
        assert_eq!(display_width("新しい"), 6);
        let apps = [
            app("a.desktop", "新しい", None),
            app("b.desktop", "Files", None),
        ];
        assert_eq!(
            apps_table(&apps, &[]),
            "新しい  a.desktop\nFiles   b.desktop\n"
        );
    }

    #[test]
    fn lists_fonts_with_the_bundled_one_first() {
        let families = [
            ("DejaVu Sans".to_string(), false),
            ("JetBrains Mono".to_string(), true),
        ];
        assert_eq!(
            fonts_table(&families, ""),
            "\
IBM Plex Mono   bundled, mono, in use
DejaVu Sans
JetBrains Mono  mono
"
        );
        assert_eq!(
            fonts_table(&families, "JetBrains Mono"),
            "\
IBM Plex Mono   bundled, mono
DejaVu Sans
JetBrains Mono  mono, in use
"
        );
    }
}
