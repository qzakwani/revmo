//! The `Exec` key: splitting it into arguments and expanding field codes.
//!
//! https://specifications.freedesktop.org/desktop-entry-spec/latest/exec-variables.html

use std::path::Path;

use serde::{Deserialize, Serialize};

/// A command line template: one entry per argument.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Exec {
    args: Vec<Arg>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Arg(Vec<Piece>);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum Piece {
    Text(String),
    Code(FieldCode),
}

/// Field codes that take a file or URL from the user.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum FieldCode {
    File,
    Files,
    Url,
    Urls,
}

impl FieldCode {
    fn from_char(c: char) -> Option<Self> {
        match c {
            'f' => Some(FieldCode::File),
            'F' => Some(FieldCode::Files),
            'u' => Some(FieldCode::Url),
            'U' => Some(FieldCode::Urls),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            FieldCode::File => "%f",
            FieldCode::Files => "%F",
            FieldCode::Url => "%u",
            FieldCode::Urls => "%U",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            FieldCode::File => "a file",
            FieldCode::Files => "files, separated by spaces",
            FieldCode::Url => "a URL",
            FieldCode::Urls => "URLs, separated by spaces",
        }
    }

    pub fn is_list(self) -> bool {
        matches!(self, FieldCode::Files | FieldCode::Urls)
    }
}

/// Values the `Exec` line's `%c`, `%i` and `%k` codes expand to.
pub struct EntryInfo<'a> {
    pub name: &'a str,
    pub icon: Option<&'a str>,
    pub path: &'a Path,
}

impl Exec {
    /// Parses an `Exec` value that has already had keyfile escapes removed.
    /// Returns `None` for an empty command or unbalanced quotes.
    pub fn parse(exec: &str, info: &EntryInfo) -> Option<Self> {
        let args = tokenize(exec, Some(info))?;
        args.first()?;
        Some(Exec { args })
    }

    /// The user-supplied field codes, in order.
    pub fn codes(&self) -> Vec<FieldCode> {
        self.args
            .iter()
            .flat_map(|a| &a.0)
            .filter_map(|p| match p {
                Piece::Code(c) => Some(*c),
                Piece::Text(_) => None,
            })
            .collect()
    }

    /// Builds the argument list. `values` has one entry per field code
    /// returned by [`Exec::codes`]; missing entries count as empty.
    pub fn expand(&self, values: &[Vec<String>]) -> Vec<String> {
        let mut values = values.iter();
        let mut argv = Vec::new();
        for arg in &self.args {
            if let [Piece::Code(code)] = arg.0.as_slice() {
                // A code on its own expands to zero, one or (for lists) many arguments.
                let value = values.next().map(Vec::as_slice).unwrap_or_default();
                if code.is_list() {
                    argv.extend(value.iter().cloned());
                } else if let Some(v) = value.first() {
                    argv.push(v.clone());
                }
                continue;
            }
            let mut out = String::new();
            for piece in &arg.0 {
                match piece {
                    Piece::Text(t) => out.push_str(t),
                    Piece::Code(_) => {
                        // Codes inside a larger argument can only take one value.
                        if let Some(v) = values.next().and_then(|v| v.first()) {
                            out.push_str(v);
                        }
                    }
                }
            }
            argv.push(out);
        }
        argv
    }
}

/// Splits what the user typed for a list field code, honoring quotes.
pub fn split_user_args(input: &str) -> Vec<String> {
    tokenize(input, None)
        .unwrap_or_else(|| {
            input
                .split_whitespace()
                .map(|a| Arg(vec![Piece::Text(a.into())]))
                .collect()
        })
        .iter()
        .map(arg_text)
        .collect()
}

fn arg_text(arg: &Arg) -> String {
    arg.0
        .iter()
        .filter_map(|p| match p {
            Piece::Text(t) => Some(t.as_str()),
            Piece::Code(_) => None,
        })
        .collect()
}

/// Splits a command line into arguments. Double quotes follow the spec
/// (`\"`, `` \` ``, `\$` and `\\` are escapes). Single quotes and backslashes
/// outside quotes are accepted too, as some files rely on shell-like syntax.
/// Field codes are only interpreted when `info` is given.
fn tokenize(input: &str, info: Option<&EntryInfo>) -> Option<Vec<Arg>> {
    let mut args = Vec::new();
    let mut pieces: Vec<Piece> = Vec::new();
    let mut text = String::new();
    // An argument exists once anything (even empty quotes) was seen.
    let mut started = false;
    let mut quote: Option<char> = None;
    let mut chars = input.chars().peekable();

    let flush = |text: &mut String, pieces: &mut Vec<Piece>| {
        if !text.is_empty() {
            pieces.push(Piece::Text(std::mem::take(text)));
        }
    };

    while let Some(c) = chars.next() {
        match (quote, c) {
            (None, c) if c.is_whitespace() => {
                if started {
                    flush(&mut text, &mut pieces);
                    args.push(Arg(std::mem::take(&mut pieces)));
                    started = false;
                }
            }
            (None, '"' | '\'') => {
                quote = Some(c);
                started = true;
            }
            (Some(q), c) if c == q => quote = None,
            (Some('"'), '\\') => match chars.peek() {
                Some(&n @ ('"' | '`' | '$' | '\\')) => {
                    chars.next();
                    text.push(n);
                }
                _ => text.push('\\'),
            },
            (None, '\\') => {
                started = true;
                if let Some(n) = chars.next() {
                    text.push(n);
                }
            }
            (Some('\''), c) => text.push(c),
            (_, '%') if info.is_some() => {
                let info = info.unwrap();
                match chars.next() {
                    Some('%') => {
                        text.push('%');
                        started = true;
                    }
                    Some(code) if FieldCode::from_char(code).is_some() => {
                        flush(&mut text, &mut pieces);
                        pieces.push(Piece::Code(FieldCode::from_char(code).unwrap()));
                        started = true;
                    }
                    Some('i') => {
                        if let Some(icon) = info.icon {
                            // %i expands to two arguments: --icon <icon>.
                            if !started {
                                args.push(Arg(vec![Piece::Text("--icon".into())]));
                            }
                            text.push_str(icon);
                            started = true;
                        }
                    }
                    Some('c') => {
                        text.push_str(info.name);
                        started = true;
                    }
                    Some('k') => {
                        text.push_str(&info.path.to_string_lossy());
                        started = true;
                    }
                    // Deprecated (%d, %n, %v, ...) and unknown codes are removed.
                    _ => {}
                }
            }
            (_, c) => {
                text.push(c);
                started = true;
            }
        }
    }
    if quote.is_some() {
        return None;
    }
    if started {
        flush(&mut text, &mut pieces);
        args.push(Arg(pieces));
    }
    Some(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> EntryInfo<'static> {
        EntryInfo {
            name: "My App",
            icon: Some("my-icon"),
            path: Path::new("/apps/my app.desktop"),
        }
    }

    fn expand(exec: &str, values: &[&[&str]]) -> Vec<String> {
        let values: Vec<Vec<String>> = values
            .iter()
            .map(|v| v.iter().map(|s| s.to_string()).collect())
            .collect();
        Exec::parse(exec, &info()).unwrap().expand(&values)
    }

    #[test]
    fn splits_and_unquotes() {
        assert_eq!(
            expand(
                r#"app  "a b" "say \"hi\" \$HOME \\ \n" 'single "q"' c\ d """#,
                &[]
            ),
            [
                "app",
                "a b",
                r#"say "hi" $HOME \ \n"#,
                r#"single "q""#,
                "c d",
                ""
            ]
        );
        assert_eq!(Exec::parse("app \"open", &info()), None);
        assert_eq!(Exec::parse("   ", &info()), None);
    }

    #[test]
    fn expands_field_codes() {
        assert_eq!(
            expand("app %F --name %c %i %k 100%% %d", &[&["a b.txt", "c.txt"]]),
            [
                "app",
                "a b.txt",
                "c.txt",
                "--name",
                "My App",
                "--icon",
                "my-icon",
                "/apps/my app.desktop",
                "100%"
            ]
        );
        // Optional codes with no value disappear entirely.
        assert_eq!(expand("app %u --flag", &[]), ["app", "--flag"]);
        assert_eq!(expand("app %f --flag", &[&["x"]]), ["app", "x", "--flag"]);
        // A code inside a larger argument takes one value.
        assert_eq!(expand("app --file=%f", &[&["x y"]]), ["app", "--file=x y"]);
        assert_eq!(expand("app --file=%f", &[]), ["app", "--file="]);
    }

    #[test]
    fn icon_code_without_icon_is_dropped() {
        let info = EntryInfo {
            icon: None,
            ..info()
        };
        let exec = Exec::parse("app %i --x", &info).unwrap();
        assert_eq!(exec.expand(&[]), ["app", "--x"]);
    }

    #[test]
    fn reports_codes() {
        let exec = Exec::parse("/usr/bin/foo --a %U %f", &info()).unwrap();
        assert_eq!(exec.codes(), [FieldCode::Urls, FieldCode::File]);
    }

    #[test]
    fn splits_user_args() {
        assert_eq!(
            split_user_args(r#"a.pdf "b c.pdf"  d"#),
            ["a.pdf", "b c.pdf", "d"]
        );
        assert_eq!(split_user_args("100%"), ["100%"]);
        assert_eq!(split_user_args("it's"), ["it's"]);
    }
}
