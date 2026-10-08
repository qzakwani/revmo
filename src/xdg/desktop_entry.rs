//! Interpretation of `.desktop` files: visibility rules, actions and the
//! `Exec` key's field codes.
//!
//! https://specifications.freedesktop.org/desktop-entry-spec/latest/

use std::path::Path;

use super::base_dirs::find_executable;
use super::keyfile::{Group, KeyFile, Locale};

const MAIN_GROUP: &str = "Desktop Entry";

/// Field codes that take a file or URL from the user; revmo asks for them on
/// the args page.
const ARG_CODES: [char; 4] = ['f', 'F', 'u', 'U'];

/// The environment an entry is evaluated against.
pub struct Context {
    pub locale: Locale,
    /// Lowercased `$XDG_CURRENT_DESKTOP` entries.
    pub desktops: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    pub exec: Exec,
    pub icon: Option<String>,
    pub actions: Vec<Action>,
}

#[derive(Debug, PartialEq)]
pub struct Action {
    pub name: String,
    pub exec: Exec,
    /// The action's own icon; callers fall back to the entry's.
    pub icon: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Exec {
    /// The command line with field codes expanded or removed.
    pub command: String,
    /// The `%f`/`%F`/`%u`/`%U` codes the command takes, in order.
    pub args: Vec<String>,
}

/// Why an entry is not shown.
#[derive(Debug, PartialEq)]
pub enum Skip {
    NoMainGroup,
    NotApplication,
    Hidden,
    NoDisplay,
    NotShownIn,
    TryExecNotFound,
    MissingName,
    MissingExec,
}

pub fn parse(contents: &str, path: &Path, ctx: &Context) -> Result<Entry, Skip> {
    parse_with(contents, path, ctx, find_executable)
}

fn parse_with(
    contents: &str,
    path: &Path,
    ctx: &Context,
    executable_exists: impl Fn(&str) -> bool,
) -> Result<Entry, Skip> {
    let file = KeyFile::parse(contents);
    let main = file.group(MAIN_GROUP).ok_or(Skip::NoMainGroup)?;

    if main.raw("Type") != Some("Application") {
        return Err(Skip::NotApplication);
    }
    if main.bool("Hidden") {
        return Err(Skip::Hidden);
    }
    if main.bool("NoDisplay") {
        return Err(Skip::NoDisplay);
    }
    if !shown_in(main, &ctx.desktops) {
        return Err(Skip::NotShownIn);
    }
    if let Some(try_exec) = main.string("TryExec") {
        if !executable_exists(&try_exec) {
            return Err(Skip::TryExecNotFound);
        }
    }

    let name = main
        .locale_string("Name", &ctx.locale)
        .filter(|n| !n.trim().is_empty())
        .ok_or(Skip::MissingName)?;
    let icon = main.string("Icon").filter(|i| !i.is_empty());
    let exec = main
        .string("Exec")
        .map(|e| expand_exec(&e, &name, icon.as_deref(), path))
        .filter(|e| !e.command.is_empty())
        .ok_or(Skip::MissingExec)?;

    let actions = main
        .list("Actions", ';')
        .iter()
        .filter_map(|id| parse_action(&file, id, path, ctx, icon.as_deref()))
        .collect();

    Ok(Entry {
        name: name.trim().to_string(),
        exec,
        icon,
        actions,
    })
}

fn parse_action(
    file: &KeyFile,
    id: &str,
    path: &Path,
    ctx: &Context,
    entry_icon: Option<&str>,
) -> Option<Action> {
    let group = file.group(&format!("Desktop Action {id}"))?;
    let name = group
        .locale_string("Name", &ctx.locale)
        .filter(|n| !n.trim().is_empty())?;
    let icon = group.string("Icon").filter(|i| !i.is_empty());
    // An action without Exec is only launchable over D-Bus, which revmo doesn't do.
    let exec = group
        .string("Exec")
        .map(|e| expand_exec(&e, &name, icon.as_deref().or(entry_icon), path))
        .filter(|e| !e.command.is_empty())?;
    Some(Action {
        name: name.trim().to_string(),
        exec,
        icon,
    })
}

fn shown_in(group: &Group, desktops: &[String]) -> bool {
    let matches = |key| {
        group
            .list(key, ';')
            .iter()
            .any(|d| desktops.contains(&d.to_lowercase()))
    };
    if group.raw("OnlyShowIn").is_some() && !matches("OnlyShowIn") {
        return false;
    }
    !matches("NotShowIn")
}

/// Expands the field codes in an (already unescaped) `Exec` value.
///
/// The command is run through `sh -c`, so expanded values are shell-quoted.
/// Deprecated and unknown codes are dropped, as the spec requires.
fn expand_exec(exec: &str, name: &str, icon: Option<&str>, path: &Path) -> Exec {
    let mut command = String::with_capacity(exec.len());
    let mut args = Vec::new();
    let mut chars = exec.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            command.push(c);
            continue;
        }
        match chars.next() {
            Some('%') => command.push('%'),
            Some(code) if ARG_CODES.contains(&code) => args.push(format!("%{code}")),
            Some('i') => {
                if let Some(icon) = icon {
                    command.push_str("--icon ");
                    command.push_str(&shell_quote(icon));
                }
            }
            Some('c') => command.push_str(&shell_quote(name)),
            Some('k') => command.push_str(&shell_quote(&path.to_string_lossy())),
            _ => {}
        }
    }
    Exec {
        // Don't collapse inner whitespace: it may be inside a quoted argument.
        command: command.trim().to_string(),
        args,
    }
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(desktops: &[&str]) -> Context {
        Context {
            locale: Locale::parse("de_DE.UTF-8"),
            desktops: desktops.iter().map(|d| d.to_lowercase()).collect(),
        }
    }

    fn parse_str(contents: &str) -> Result<Entry, Skip> {
        parse_with(
            contents,
            Path::new("/apps/foo.desktop"),
            &ctx(&["GNOME"]),
            |p| p == "present",
        )
    }

    fn exec(command: &str, args: &[&str]) -> Exec {
        Exec {
            command: command.into(),
            args: args.iter().map(|a| a.to_string()).collect(),
        }
    }

    #[test]
    fn parses_entry_with_actions() {
        let entry = parse_str(
            "[Desktop Entry]
Type=Application
GenericName=Web Browser
TryExec=present
Name=Firefox
Name[de]=Feuerfuchs
Exec=firefox %u
Icon=firefox
Actions=new-window;private;missing;

[Desktop Action new-window]
Name=New Window
Exec=firefox --new-window %u

[Desktop Action private]
Name=Private
Exec=firefox --private-window
Icon=firefox-private
",
        )
        .unwrap();

        assert_eq!(
            entry,
            Entry {
                name: "Feuerfuchs".into(),
                exec: exec("firefox", &["%u"]),
                icon: Some("firefox".into()),
                actions: vec![
                    Action {
                        name: "New Window".into(),
                        exec: exec("firefox --new-window", &["%u"]),
                        icon: None,
                    },
                    Action {
                        name: "Private".into(),
                        exec: exec("firefox --private-window", &[]),
                        icon: Some("firefox-private".into()),
                    },
                ],
            }
        );
    }

    #[test]
    fn ignores_actions_not_listed_and_other_groups() {
        let entry = parse_str(
            "[Desktop Entry]
Type=Application
Name=App
Exec=app

[X-Vendor Group]
Exec=evil

[Desktop Action unlisted]
Name=Unlisted
Exec=app --unlisted
",
        )
        .unwrap();
        assert_eq!(entry.exec, exec("app", &[]));
        assert!(entry.actions.is_empty());
    }

    #[test]
    fn applies_visibility_rules() {
        let base = "[Desktop Entry]\nName=App\nExec=app\n";
        let check = |extra: &str| parse_str(&format!("{base}{extra}")).err();

        assert_eq!(check("Type=Application\n"), None);
        assert_eq!(check("Type=Link\n"), Some(Skip::NotApplication));
        assert_eq!(check(""), Some(Skip::NotApplication));
        assert_eq!(check("Type=Application\nHidden=true\n"), Some(Skip::Hidden));
        assert_eq!(
            check("Type=Application\nNoDisplay=true\n"),
            Some(Skip::NoDisplay)
        );
        assert_eq!(
            check("Type=Application\nOnlyShowIn=KDE;\n"),
            Some(Skip::NotShownIn)
        );
        assert_eq!(check("Type=Application\nOnlyShowIn=KDE;GNOME;\n"), None);
        assert_eq!(
            check("Type=Application\nNotShowIn=GNOME;\n"),
            Some(Skip::NotShownIn)
        );
        assert_eq!(
            check("Type=Application\nTryExec=absent\n"),
            Some(Skip::TryExecNotFound)
        );
        assert_eq!(
            parse_str("[Desktop Entry]\nType=Application\nName=App\n").err(),
            Some(Skip::MissingExec)
        );
        assert_eq!(
            parse_str("[Other]\nName=App\n").err(),
            Some(Skip::NoMainGroup)
        );
    }

    #[test]
    fn expands_field_codes() {
        let path = Path::new("/apps/it's.desktop");
        assert_eq!(
            expand_exec(
                "app %F --name %c %i %k 100%% %d %x",
                "My App",
                Some("app-icon"),
                path
            ),
            exec(
                r#"app  --name 'My App' --icon 'app-icon' '/apps/it'\''s.desktop' 100%"#,
                &["%F"]
            )
        );
        assert_eq!(
            expand_exec("app %i %U %f", "A", None, path),
            exec("app", &["%U", "%f"])
        );
    }

    #[test]
    fn keeps_exec_quoting_and_unescapes_once() {
        // `\\\\` in the file is `\\` after string unescaping, which sh reads as one `\`.
        let entry = parse_str(
            "[Desktop Entry]\nType=Application\nName=App\nExec=sh -c \"echo \\\\\\\\$HOME\"\n",
        )
        .unwrap();
        assert_eq!(entry.exec.command, r#"sh -c "echo \\$HOME""#);
    }
}
