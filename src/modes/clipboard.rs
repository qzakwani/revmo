//! Clipboard mode: the clipboard history as a menu (see [`super::menu`]).
//! Revmo only shows it; the user's clipboard manager (cliphist, greenclip,
//! ...) keeps it. `ClipboardList=` in the config prints the history, one
//! entry per line, and `ClipboardPick=` gets the picked line on stdin and
//! copies that entry. Both are run by the shell, so they're written as
//! in a terminal, pipes and all (`cliphist decode | wl-copy`); the picked
//! line only ever goes to stdin, never into the command.

use std::io::{self, Write};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Typed first in the launcher's search box, opens clipboard mode, where
/// it's shown as the mode's sign.
pub const PREFIX: char = ';';

/// How long to wait for the pick command to report a failure. Some copy
/// tools stay running to serve the clipboard, so a command still running
/// after this counts as done.
const PICK_WAIT: Duration = Duration::from_secs(1);

/// The history's lines, from running `command`.
pub fn list(command: &str) -> Result<Vec<String>, String> {
    let output = command_for(command, "ClipboardList")?
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("can't run ClipboardList: {e}"))?;
    if !output.status.success() {
        return Err(failure("ClipboardList", &output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(String::from)
        .collect())
}

/// Hands `line` to `command` on stdin. Waits only to see it didn't fail
/// right away (see [`PICK_WAIT`]); it keeps running if it needs to.
pub fn pick(command: &str, line: &str) -> Result<(), String> {
    let mut child = command_for(command, "ClipboardPick")?
        .stdin(Stdio::piped())
        // Not piped: copy tools that stay running in the background would
        // keep a pipe open, and reading it would never end.
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        // Own process group, so it isn't stopped along with revmo.
        .process_group(0)
        .spawn()
        .map_err(|e| format!("can't run ClipboardPick: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        // Dropped at the end, so the command sees the input end. A broken
        // pipe means it chose not to read it all; its exit status says
        // whether that's a failure.
        match writeln!(stdin, "{line}") {
            Err(e) if e.kind() != io::ErrorKind::BrokenPipe => {
                return Err(format!("can't send the entry to ClipboardPick: {e}"));
            }
            _ => {}
        }
    }
    let started = Instant::now();
    while started.elapsed() < PICK_WAIT {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => return Err(format!("ClipboardPick failed ({status})")),
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => return Ok(()),
        }
    }
    Ok(())
}

/// `command` run by the shell, or why it can't run.
fn command_for(command: &str, key: &str) -> Result<Command, String> {
    if command.trim().is_empty() {
        return Err(format!(
            "set {key}= in the config to use the clipboard (see revmo --help)"
        ));
    }
    let mut sh = Command::new("sh");
    sh.arg("-c").arg(command);
    Ok(sh)
}

/// "`key` failed", with the first line of what it printed on stderr.
fn failure(key: &str, stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    match stderr.lines().map(str::trim).find(|l| !l.is_empty()) {
        Some(reason) => format!("{key} failed: {reason}"),
        None => format!("{key} failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_lines_and_reports_failures() {
        assert_eq!(
            list("printf 'b\\na b\\n'"),
            Ok(vec!["b".to_string(), "a b".to_string()])
        );
        assert_eq!(
            list("echo nope >&2; exit 3"),
            Err("ClipboardList failed: nope".to_string())
        );
        assert!(list("").unwrap_err().starts_with("set ClipboardList="));
        assert!(list("   ").unwrap_err().starts_with("set ClipboardList="));
        let missing = list("revmo-no-such-program").unwrap_err();
        assert!(missing.starts_with("ClipboardList failed: "), "{missing}");
        assert!(missing.contains("revmo-no-such-program"), "{missing}");
    }

    #[test]
    fn pick_gets_the_line_on_stdin_through_a_pipe() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("out");
        let command = format!("tr a-z A-Z | cat > '{}'", out.display());
        // Shell syntax in the line stays text: it only goes to stdin.
        pick(&command, "42\tsome $(text)").unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "42\tSOME $(TEXT)\n");
        assert_eq!(
            pick("exit 1", "x"),
            Err("ClipboardPick failed (exit status: 1)".to_string())
        );
        assert!(pick("", "x").is_err());
    }
}
