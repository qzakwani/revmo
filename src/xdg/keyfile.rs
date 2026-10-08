//! Parser for the freedesktop "key file" format shared by `.desktop` files
//! and icon theme `index.theme` files.
//!
//! https://specifications.freedesktop.org/desktop-entry-spec/latest/basic-format.html

use std::collections::HashMap;
use std::env;

#[derive(Debug, Default)]
pub struct KeyFile {
    groups: Vec<Group>,
}

#[derive(Debug)]
pub struct Group {
    pub name: String,
    entries: HashMap<String, String>,
}

impl KeyFile {
    pub fn parse(contents: &str) -> Self {
        let mut groups: Vec<Group> = Vec::new();
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                groups.push(Group {
                    name: name.to_string(),
                    entries: HashMap::new(),
                });
                continue;
            }
            // Key/value pairs before the first group header are invalid; skip them.
            let (Some(group), Some((key, value))) = (groups.last_mut(), line.split_once('='))
            else {
                continue;
            };
            // Duplicate keys are invalid; keep the first occurrence.
            group
                .entries
                .entry(key.trim_end().to_string())
                .or_insert_with(|| value.trim_start().to_string());
        }
        KeyFile { groups }
    }

    /// Returns the first group with the given name.
    pub fn group(&self, name: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.name == name)
    }
}

impl Group {
    /// The raw, unescaped value of `key`.
    pub fn raw(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    pub fn string(&self, key: &str) -> Option<String> {
        self.raw(key).map(unescape)
    }

    pub fn bool(&self, key: &str) -> bool {
        self.raw(key) == Some("true")
    }

    pub fn number(&self, key: &str) -> Option<u32> {
        self.raw(key).and_then(|v| v.trim().parse().ok())
    }

    /// A list value split on `sep`. Empty items are dropped and `\<sep>`
    /// escapes a literal separator.
    pub fn list(&self, key: &str, sep: char) -> Vec<String> {
        let Some(raw) = self.raw(key) else {
            return Vec::new();
        };
        let mut items = Vec::new();
        let mut current = String::new();
        let mut chars = raw.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some(n) if n == sep => current.push(sep),
                    Some(n) => {
                        current.push('\\');
                        current.push(n);
                    }
                    None => current.push('\\'),
                }
            } else if c == sep {
                items.push(unescape(&current));
                current.clear();
            } else {
                current.push(c);
            }
        }
        items.push(unescape(&current));
        items.retain(|s| !s.is_empty());
        items
    }

    /// The best match for a localized key (`Name[de_DE]`, `Name[de]`, ...),
    /// falling back to the unlocalized key.
    pub fn locale_string(&self, key: &str, locale: &Locale) -> Option<String> {
        locale
            .candidates()
            .iter()
            .find_map(|l| self.string(&format!("{key}[{l}]")))
            .or_else(|| self.string(key))
    }
}

/// Expands the escape sequences allowed in string values.
pub fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('s') => out.push(' '),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// A POSIX locale (`lang_COUNTRY.ENCODING@MODIFIER`) used to pick localized keys.
#[derive(Debug, Default, Clone)]
pub struct Locale {
    lang: Option<String>,
    country: Option<String>,
    modifier: Option<String>,
}

impl Locale {
    pub fn from_env() -> Self {
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|var| env::var(var).ok())
            .find(|v| !v.is_empty())
            .map(|v| Locale::parse(&v))
            .unwrap_or_default()
    }

    pub fn parse(value: &str) -> Self {
        let (rest, modifier) = match value.split_once('@') {
            Some((r, m)) => (r, Some(m.to_string())),
            None => (value, None),
        };
        let rest = rest.split('.').next().unwrap_or("");
        let (lang, country) = match rest.split_once('_') {
            Some((l, c)) => (l, Some(c.to_string())),
            None => (rest, None),
        };
        if lang.is_empty() || lang == "C" || lang == "POSIX" {
            return Locale::default();
        }
        Locale {
            lang: Some(lang.to_string()),
            country,
            modifier,
        }
    }

    /// Locale keys to try, most specific first, in the order the spec defines.
    fn candidates(&self) -> Vec<String> {
        let Some(lang) = &self.lang else {
            return Vec::new();
        };
        let mut out = Vec::with_capacity(4);
        if let (Some(c), Some(m)) = (&self.country, &self.modifier) {
            out.push(format!("{lang}_{c}@{m}"));
        }
        if let Some(c) = &self.country {
            out.push(format!("{lang}_{c}"));
        }
        if let Some(m) = &self.modifier {
            out.push(format!("{lang}@{m}"));
        }
        out.push(lang.clone());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_groups_and_ignores_comments() {
        let kf = KeyFile::parse(
            "# comment\nstray=1\n[Desktop Entry]\nName = Foo\nName=Dup\n\n[Desktop Action new]\nName=New\n",
        );
        let main = kf.group("Desktop Entry").unwrap();
        assert_eq!(main.raw("Name"), Some("Foo"));
        assert_eq!(
            kf.group("Desktop Action new").unwrap().raw("Name"),
            Some("New")
        );
        assert!(kf.group("stray").is_none());
    }

    #[test]
    fn unescapes_values() {
        assert_eq!(unescape(r"a\sb\\c\td"), "a b\\c\td");
        assert_eq!(unescape(r#"say \"hi\""#), r#"say \"hi\""#);
    }

    #[test]
    fn splits_lists() {
        let kf = KeyFile::parse("[G]\nA=one;two\\;three;;four;\nB=x,y\n");
        let g = kf.group("G").unwrap();
        assert_eq!(g.list("A", ';'), ["one", "two;three", "four"]);
        assert_eq!(g.list("B", ','), ["x", "y"]);
        assert!(g.list("missing", ';').is_empty());
    }

    #[test]
    fn picks_most_specific_locale() {
        let kf = KeyFile::parse(
            "[G]\nName=Default\nName[de]=Deutsch\nName[de_AT]=Österreich\nName[sr@latin]=Latinica\n",
        );
        let g = kf.group("G").unwrap();
        let get = |l: &str| g.locale_string("Name", &Locale::parse(l)).unwrap();
        assert_eq!(get("de_AT.UTF-8"), "Österreich");
        assert_eq!(get("de_DE.UTF-8"), "Deutsch");
        assert_eq!(get("sr_RS@latin"), "Latinica");
        assert_eq!(get("fr_FR.UTF-8"), "Default");
        assert_eq!(get("C.UTF-8"), "Default");
    }
}
