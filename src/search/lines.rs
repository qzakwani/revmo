//! Filtering menu lines (`revmo menu`, clipboard history) the way dmenu
//! does. A line matches when it contains every word of the query, ignoring
//! case. Lines equal to the query come first, then lines starting with it,
//! then lines containing it as one run, then the rest. Each group keeps
//! the input order, which often means something: clipboard history, for
//! one, lists the newest first.

/// Only this much of a line is searched, so long pastes stay cheap.
const SEARCHED_CHARS: usize = 2048;

/// The lines and which of them match the query, kept up to date as either
/// changes. Lines added while a query is typed only cost their own check.
#[derive(Debug, Default)]
pub struct LineSearch {
    /// Each line's searched part, lowercased.
    keys: Vec<String>,
    /// The current query, lowercased and cut like the keys.
    query: String,
    /// Indices of the matching lines by group, best first, each ascending.
    groups: [Vec<usize>; 4],
}

impl LineSearch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds lines after the ones already there. Only the new lines are
    /// checked; each joins the end of its group. Returns how many joined
    /// each group.
    pub fn push<S: AsRef<str>>(&mut self, lines: &[S]) -> [usize; 4] {
        let start = self.keys.len();
        self.keys.extend(lines.iter().map(|l| lower(l.as_ref())));
        let before = self.groups.each_ref().map(Vec::len);
        for i in start..self.keys.len() {
            if let Some(group) = group(&self.keys[i], &self.query) {
                self.groups[group].push(i);
            }
        }
        std::array::from_fn(|g| self.groups[g].len() - before[g])
    }

    /// Drops every line, keeping the query.
    pub fn clear(&mut self) {
        self.keys.clear();
        self.groups = Default::default();
    }

    /// Filters for `query`. When it extends the previous query, only the
    /// lines that matched that one can match, so only those are checked.
    pub fn set_query(&mut self, query: &str) {
        let query = lower(query);
        let candidates: Vec<usize> = if query.starts_with(&self.query) {
            let mut matched = self.groups.concat();
            matched.sort_unstable();
            matched
        } else {
            (0..self.keys.len()).collect()
        };
        self.query = query;
        let mut groups: [Vec<usize>; 4] = Default::default();
        for i in candidates {
            if let Some(group) = group(&self.keys[i], &self.query) {
                groups[group].push(i);
            }
        }
        self.groups = groups;
    }

    /// How many lines match.
    pub fn len(&self) -> usize {
        self.groups.iter().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The index of the matching line at `position`, best first.
    pub fn get(&self, position: usize) -> Option<usize> {
        let (group, offset) = self.locate(position)?;
        Some(self.groups[group][offset])
    }

    /// Which group `position` falls in, and where in it.
    pub fn locate(&self, mut position: usize) -> Option<(usize, usize)> {
        for (g, group) in self.groups.iter().enumerate() {
            if position < group.len() {
                return Some((g, position));
            }
            position -= group.len();
        }
        None
    }

    /// Where the line with index `line` is listed, if it matches.
    pub fn position_of(&self, line: usize) -> Option<usize> {
        let mut before = 0;
        for group in &self.groups {
            if let Ok(offset) = group.binary_search(&line) {
                return Some(before + offset);
            }
            before += group.len();
        }
        None
    }

    /// Indices of the matching lines, best first.
    pub fn ranked(&self) -> impl Iterator<Item = usize> + '_ {
        self.groups.iter().flatten().copied()
    }
}

/// Lowercased per character (so "Σ" is always "σ", wherever it is, as the
/// highlighting has it) and cut to [`SEARCHED_CHARS`]. Queries are cut too,
/// so a line still matches itself when it's put in the input whole.
fn lower(text: &str) -> String {
    text.chars()
        .take(SEARCHED_CHARS)
        .flat_map(char::to_lowercase)
        .collect()
}

/// Which group `key` belongs in for `query`, or `None` if some word of the
/// query isn't in it.
fn group(key: &str, query: &str) -> Option<usize> {
    if !query.split_whitespace().all(|w| key.contains(w)) {
        return None;
    }
    let query = query.trim();
    Some(if query.is_empty() || key == query {
        0
    } else if key.starts_with(query) {
        1
    } else if key.contains(query) {
        2
    } else {
        3
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search(lines: &[&str], query: &str) -> Vec<usize> {
        let mut s = LineSearch::new();
        s.push(lines);
        s.set_query(query);
        s.ranked().collect()
    }

    #[test]
    fn every_word_must_appear_ignoring_case() {
        let lines = ["Cargo build", "cargo test", "git commit", "BUILD cargo"];
        assert_eq!(search(&lines, "CARGO build"), [0, 3]);
        assert_eq!(search(&lines, "car bu"), [0, 3]);
        assert_eq!(search(&lines, "carg ommit"), Vec::<usize>::new());
        // No fuzzy matching: letters out of a word don't count.
        assert_eq!(search(&lines, "cb"), Vec::<usize>::new());
    }

    #[test]
    fn empty_query_keeps_input_order() {
        assert_eq!(search(&["b", "a", "c"], ""), [0, 1, 2]);
        assert_eq!(search(&["b", "a", "c"], "   "), [0, 1, 2]);
    }

    #[test]
    fn exact_then_prefix_then_run_then_words() {
        let lines = [
            "cargo, then test", // the words, apart
            "test cargo",       // the words, apart
            "run cargo test",   // contains the query
            "cargo test --all", // starts with it
            "cargo test",       // equals it
            "cargo testing",    // starts with it
        ];
        assert_eq!(search(&lines, "cargo test"), [4, 3, 5, 2, 0, 1]);
    }

    #[test]
    fn narrowing_and_widening_agree_with_a_fresh_search() {
        let lines = ["alpha beta", "alphabet", "beta", "Alpine", "gamma alp"];
        let mut s = LineSearch::new();
        s.push(&lines);
        for query in [
            "a", "al", "alp", "alph", "alph b", "alp", "", "beta", "x", "be",
        ] {
            s.set_query(query);
            assert_eq!(
                s.ranked().collect::<Vec<_>>(),
                search(&lines, query),
                "{query:?}"
            );
        }
    }

    #[test]
    fn lines_added_later_are_filtered_and_ranked() {
        let ranked = |s: &LineSearch| s.ranked().collect::<Vec<_>>();
        let mut s = LineSearch::new();
        s.push(&["xx foo", "bar"]);
        s.set_query("foo");
        assert_eq!(ranked(&s), [0]);
        assert_eq!(s.push(&["foo first", "nope", "foo"]), [1, 1, 0, 0]);
        assert_eq!(ranked(&s), [4, 2, 0]);
        assert_eq!((s.get(1), s.get(3)), (Some(2), None));
        assert_eq!(s.locate(2), Some((2, 0)));
        assert_eq!((s.position_of(0), s.position_of(1)), (Some(2), None));
        s.clear();
        assert!(s.is_empty());
        s.push(&["a foo"]);
        assert_eq!(ranked(&s), [0]);
    }

    #[test]
    fn a_long_line_still_matches_itself() {
        let long = "x".repeat(SEARCHED_CHARS + 100);
        assert_eq!(search(&[&long], &long), [0]);
    }

    #[test]
    fn sigma_matches_wherever_it_is() {
        assert_eq!(search(&["ΟΔΟΣΤ"], "ΟΔΟΣ"), [0]);
        assert_eq!(search(&["ΟΔΟΣ"], "οδοσ"), [0]);
    }

    #[test]
    fn only_the_start_of_long_lines_is_searched() {
        let long = format!("{}needle", "x".repeat(SEARCHED_CHARS));
        assert!(search(&[&long], "needle").is_empty());
        let short = format!("{}needle", "x".repeat(SEARCHED_CHARS - 6));
        assert_eq!(search(&[&short], "needle"), [0]);
    }
}
