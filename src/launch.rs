//! Starting programs: no shell involved, detached from revmo.

use std::env;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::xdg::base_dirs::find_executable;

/// Terminal emulators to try for `Terminal=true` entries, with the
/// arguments that come before the command to run.
const TERMINALS: [(&str, &[&str]); 12] = [
    ("xdg-terminal-exec", &[]),
    ("kitty", &[]),
    ("foot", &[]),
    ("alacritty", &["-e"]),
    ("wezterm", &["start", "--"]),
    ("ghostty", &["-e"]),
    ("gnome-terminal", &["--"]),
    ("konsole", &["-e"]),
    ("xfce4-terminal", &["-x"]),
    ("tilix", &["-e"]),
    ("x-terminal-emulator", &["-e"]),
    ("xterm", &["-e"]),
];

/// Spawns `argv`, in a terminal emulator when `terminal` is set.
pub fn spawn(argv: Vec<String>, terminal: bool, working_dir: Option<&str>) -> io::Result<()> {
    let argv = if terminal {
        let mut prefix = terminal_command()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no terminal emulator found"))?;
        prefix.extend(argv);
        prefix
    } else {
        argv
    };
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty command"))?;

    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        // Own process group, so signals aimed at revmo's group (e.g. the
        // terminal it was started from closing) don't reach the app.
        .process_group(0);
    // The spec says to use Path= as the working directory; ignore it if it
    // doesn't exist rather than failing to launch.
    if let Some(dir) = working_dir.filter(|d| Path::new(d).is_dir()) {
        command.current_dir(dir);
    }
    command.spawn().map(|_| ())
}

/// The command prefix that runs a program in a terminal: `$TERMINAL -e`
/// when set, otherwise the first emulator found in `TERMINALS`.
fn terminal_command() -> Option<Vec<String>> {
    if let Some(term) = env::var("TERMINAL").ok().filter(|t| !t.trim().is_empty()) {
        let mut cmd: Vec<String> = term.split_whitespace().map(String::from).collect();
        if find_executable(&cmd[0]) {
            let known = TERMINALS.iter().find(|(name, _)| cmd[0].ends_with(name));
            match known {
                Some((_, args)) => cmd.extend(args.iter().map(|a| a.to_string())),
                None => cmd.push("-e".into()),
            }
            return Some(cmd);
        }
    }
    TERMINALS
        .iter()
        .find(|(name, _)| find_executable(name))
        .map(|(name, args)| {
            std::iter::once(name.to_string())
                .chain(args.iter().map(|a| a.to_string()))
                .collect()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawns_without_a_shell() {
        let _guard = crate::fork_lock();
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("out");
        // `$1` stays literal unless something hands the line to a shell twice.
        spawn(
            vec![
                "sh".into(),
                "-c".into(),
                "pwd > \"$1\"; echo \"$2\" >> \"$1\"".into(),
                "sh".into(),
                out.to_string_lossy().into(),
                "a b; $(echo injected)".into(),
            ],
            false,
            Some(tmp.path().to_str().unwrap()),
        )
        .unwrap();
        // The child is detached; wait for it to write.
        for _ in 0..100 {
            if std::fs::read_to_string(&out).is_ok_and(|s| s.lines().count() == 2) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let written = std::fs::read_to_string(&out).unwrap();
        let dir = tmp.path().canonicalize().unwrap();
        assert_eq!(
            written,
            format!("{}\na b; $(echo injected)\n", dir.display())
        );
    }

    #[test]
    fn missing_program_is_an_error() {
        let _guard = crate::fork_lock();
        assert!(spawn(vec!["revmo-no-such-program".into()], false, None).is_err());
        assert!(spawn(vec![], false, None).is_err());
    }
}
