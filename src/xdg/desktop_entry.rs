//! Interpretation of `.desktop` files: visibility rules and actions.
//!
//! https://specifications.freedesktop.org/desktop-entry-spec/latest/

use std::path::Path;

use super::base_dirs::find_executable;
use super::exec::{EntryInfo, Exec};
use super::keyfile::{Group, KeyFile, Locale};

const MAIN_GROUP: &str = "Desktop Entry";

/// The environment an entry is evaluated against.
pub struct Context {
    pub locale: Locale,
    /// Lowercased `$XDG_CURRENT_DESKTOP` entries.
    pub desktops: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    /// e.g. "Web Browser".
    pub generic_name: Option<String>,
    pub keywords: Vec<String>,
    pub exec: Exec,
    pub icon: Option<String>,
    /// `Terminal=true`: the program needs a terminal emulator.
    pub terminal: bool,
    /// `Path=`: the working directory to start the program in.
    pub working_dir: Option<String>,
    pub actions: Vec<Action>,
}

#[derive(Debug, PartialEq)]
pub struct Action {
    /// The id from `Actions=`, e.g. `new-window`.
    pub id: String,
    pub name: String,
    pub exec: Exec,
    /// The action's own icon; callers fall back to the entry's.
    pub icon: Option<String>,
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
    let info = EntryInfo {
        name: &name,
        icon: icon.as_deref(),
        path,
    };
    let exec = main
        .string("Exec")
        .and_then(|e| Exec::parse(&e, &info))
        .ok_or(Skip::MissingExec)?;

    let actions = main
        .list("Actions", ';')
        .iter()
        .filter_map(|id| parse_action(&file, id, path, ctx, icon.as_deref()))
        .collect();

    Ok(Entry {
        name: name.trim().to_string(),
        generic_name: main
            .locale_string("GenericName", &ctx.locale)
            .filter(|n| !n.trim().is_empty()),
        keywords: main.locale_list("Keywords", ';', &ctx.locale),
        exec,
        icon,
        terminal: main.bool("Terminal"),
        working_dir: main.string("Path").filter(|p| !p.is_empty()),
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
    let info = EntryInfo {
        name: &name,
        icon: icon.as_deref().or(entry_icon),
        path,
    };
    let exec = group.string("Exec").and_then(|e| Exec::parse(&e, &info))?;
    Some(Action {
        id: id.to_string(),
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

    fn argv(exec: &Exec) -> Vec<String> {
        exec.expand(&[])
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
Path=/tmp
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

        assert_eq!(entry.name, "Feuerfuchs");
        assert_eq!(entry.generic_name.as_deref(), Some("Web Browser"));
        assert_eq!(argv(&entry.exec), ["firefox"]);
        assert_eq!(entry.exec.codes().len(), 1);
        assert_eq!(entry.icon.as_deref(), Some("firefox"));
        assert!(!entry.terminal);
        assert_eq!(entry.working_dir.as_deref(), Some("/tmp"));

        let actions: Vec<_> = entry
            .actions
            .iter()
            .map(|a| {
                (
                    a.id.as_str(),
                    a.name.as_str(),
                    argv(&a.exec),
                    a.icon.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            actions,
            [
                (
                    "new-window",
                    "New Window",
                    vec!["firefox".to_string(), "--new-window".into()],
                    None
                ),
                (
                    "private",
                    "Private",
                    vec!["firefox".into(), "--private-window".into()],
                    Some("firefox-private")
                ),
            ]
        );
    }

    #[test]
    fn ignores_actions_not_listed_and_other_groups() {
        let entry = parse_str(
            "[Desktop Entry]
Type=Application
Name=App
Exec=app
Terminal=true

[X-Vendor Group]
Exec=evil

[Desktop Action unlisted]
Name=Unlisted
Exec=app --unlisted
",
        )
        .unwrap();
        assert_eq!(argv(&entry.exec), ["app"]);
        assert!(entry.terminal);
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
            parse_str("[Desktop Entry]\nType=Application\nName=App\nExec=app \"open\n").err(),
            Some(Skip::MissingExec)
        );
        assert_eq!(
            parse_str("[Other]\nName=App\n").err(),
            Some(Skip::NoMainGroup)
        );
    }

    #[test]
    fn unescapes_strings_before_exec_quoting() {
        // `\\\\` in the file is `\\` after string unescaping, which the Exec
        // quoting rules turn into one `\`.
        let entry = parse_str(
            "[Desktop Entry]\nType=Application\nName=App\nExec=sh -c \"echo \\\\\\\\$HOME\"\n",
        )
        .unwrap();
        assert_eq!(argv(&entry.exec), ["sh", "-c", "echo \\$HOME"]);
    }
}
