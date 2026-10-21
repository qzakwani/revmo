//! Showing only some columns of a menu's lines, like fzf's `--with-nth`:
//! `cliphist list` prints `42<tab>text`, and only the text is worth
//! showing, while the pick must still be the whole line for `cliphist
//! decode` to find the entry.

use std::borrow::Cow;

/// One part of a columns spec, numbered from 1.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Field {
    /// `2`: that column.
    One(usize),
    /// `2..`: that column to the end of the line, separators and all.
    From(usize),
}

/// Which columns of each line to show, and what separates them.
#[derive(Debug, Clone, PartialEq)]
pub struct Columns {
    separator: String,
    fields: Vec<Field>,
}

impl Columns {
    /// From a spec like `2`, `1,3` or `2..`, and the separator (a tab if
    /// `None`). The error says what's wrong, without naming the setting.
    pub fn parse(spec: &str, separator: Option<&str>) -> Result<Self, String> {
        let separator = separator.unwrap_or("\t");
        if separator.is_empty() {
            return Err("the separator is empty".into());
        }
        let field = |item: &str| {
            let item = item.trim();
            let (number, from) = match item.strip_suffix("..") {
                Some(number) => (number, true),
                None => (item, false),
            };
            match number.parse::<usize>() {
                Ok(n) if n > 0 && from => Some(Field::From(n)),
                Ok(n) if n > 0 => Some(Field::One(n)),
                _ => None,
            }
        };
        let fields: Option<Vec<Field>> = spec.split(',').map(field).collect();
        match fields {
            Some(fields) => Ok(Columns {
                separator: separator.to_string(),
                fields,
            }),
            None => Err(format!(
                "\"{spec}\" isn't a list of columns like 2, 1,3 or 2.."
            )),
        }
    }

    /// What's shown of `line`: its columns that the spec names, joined by
    /// the separator. A line without any of them is shown whole.
    pub fn shown<'a>(&self, line: &'a str) -> Cow<'a, str> {
        // Where each column starts and ends in the line.
        let mut spans = Vec::new();
        let mut start = 0;
        for (at, _) in line.match_indices(self.separator.as_str()) {
            spans.push((start, at));
            start = at + self.separator.len();
        }
        spans.push((start, line.len()));

        let parts: Vec<&str> = self
            .fields
            .iter()
            .filter_map(|field| match *field {
                Field::One(n) => spans.get(n - 1).map(|&(s, e)| &line[s..e]),
                Field::From(n) => spans.get(n - 1).map(|&(s, _)| &line[s..]),
            })
            .collect();
        match parts.as_slice() {
            [] => Cow::Borrowed(line),
            [one] => Cow::Borrowed(one),
            _ => Cow::Owned(parts.join(&self.separator)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(spec: &str, separator: Option<&str>, line: &str) -> String {
        Columns::parse(spec, separator)
            .unwrap()
            .shown(line)
            .into_owned()
    }

    #[test]
    fn shows_the_named_columns() {
        let line = "42\tgit push\torigin";
        assert_eq!(shown("2", None, line), "git push");
        assert_eq!(shown("2..", None, line), "git push\torigin");
        assert_eq!(shown("1,3", None, line), "42\torigin");
        assert_eq!(shown("3, 1", None, line), "origin\t42");
        assert_eq!(shown("1..", None, line), line);
    }

    #[test]
    fn other_separators() {
        assert_eq!(shown("2..", Some(": "), "7: a: b"), "a: b");
        assert_eq!(shown("2", Some(","), "a,,c"), "");
        assert_eq!(shown("3", Some(","), "a,,c"), "c");
    }

    #[test]
    fn lines_without_the_columns_show_whole() {
        assert_eq!(shown("2..", None, "no tabs here"), "no tabs here");
        assert_eq!(shown("2", None, ""), "");
        // Some columns there, others not: what's there is shown.
        assert_eq!(shown("2,5", None, "a\tb"), "b");
    }

    #[test]
    fn rejects_bad_specs() {
        for bad in ["", "0", "x", "2..3", "-1", "1,,2", "..2"] {
            assert!(Columns::parse(bad, None).is_err(), "{bad:?}");
        }
        assert!(Columns::parse("2", Some("")).is_err());
    }
}
