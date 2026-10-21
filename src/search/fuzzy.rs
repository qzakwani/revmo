//! Fuzzy matching, for apps.
//!
//! A query matches a text when its characters appear in order (not
//! necessarily together). Matches score higher when they start words,
//! run consecutively and begin near the start, so "lw" finds
//! "LibreOffice Writer" and "ff" finds "Firefox".

const MATCH: i32 = 16;
const CONSECUTIVE: i32 = 14;
const WORD_START: i32 = 10;
const TEXT_START: i32 = 12;
const GAP: i32 = 1;
const LEADING_GAP_CAP: i32 = 12;

/// One searchable text, lowercased, with its weight and per-character
/// word-start bonuses.
struct Field {
    chars: Vec<char>,
    bonus: Vec<i32>,
    weight: f64,
}

/// The searchable texts of one item.
pub struct Haystack {
    fields: Vec<Field>,
}

impl Haystack {
    /// `fields` pairs each text with a weight; the name should be 1.0 and
    /// less important texts lower.
    pub fn new<'a>(fields: impl IntoIterator<Item = (&'a str, f64)>) -> Self {
        let fields = fields
            .into_iter()
            .filter(|(text, _)| !text.is_empty())
            .map(|(text, weight)| {
                let original: Vec<char> = text.chars().collect();
                let bonus = (0..original.len())
                    .map(|i| match i {
                        0 => TEXT_START,
                        _ if is_word_start(original[i - 1], original[i]) => WORD_START,
                        _ => 0,
                    })
                    .collect();
                Field {
                    chars: original.iter().map(|&c| lower(c)).collect(),
                    bonus,
                    weight,
                }
            })
            .collect();
        Haystack { fields }
    }

    /// The match score, or `None` if some word of the query matches no
    /// field. Every whitespace-separated word must match somewhere; an empty
    /// query matches everything with score 0.
    pub fn score(&self, query: &str) -> Option<f64> {
        query
            .split_whitespace()
            .map(|word| {
                let word: Vec<char> = word.chars().map(lower).collect();
                self.fields
                    .iter()
                    .filter_map(|f| score_field(&word, f).map(|s| s as f64 * f.weight))
                    .max_by(f64::total_cmp)
            })
            .sum()
    }
}

/// Which characters of `text` (by char index, ascending) to highlight for
/// `query`; display only, ranking is [`Haystack::score`]'s job. Each word of
/// the query is shown as a run of consecutive characters if it appears in
/// `text`, preferring one that starts a word, and otherwise, if `letters`,
/// letter by letter at the earliest positions. A word not found in `text`
/// (it matched some other field) highlights nothing.
pub fn highlight(text: &str, query: &str, letters: bool) -> Vec<usize> {
    let original: Vec<char> = text.chars().collect();
    let chars: Vec<char> = original.iter().map(|&c| lower(c)).collect();
    let starts_word = |i: usize| i == 0 || is_word_start(original[i - 1], original[i]);
    let mut marked = Vec::new();
    for word in query.split_whitespace() {
        let word: Vec<char> = word.chars().map(lower).collect();
        if word.len() > chars.len() {
            continue;
        }
        let runs: Vec<usize> = (0..=chars.len() - word.len())
            .filter(|&i| chars[i..i + word.len()] == word[..])
            .collect();
        if let Some(&start) = runs.iter().find(|&&i| starts_word(i)).or(runs.first()) {
            marked.extend(start..start + word.len());
            continue;
        }
        if !letters {
            continue;
        }
        let mut found = Vec::with_capacity(word.len());
        let mut letters = word.iter().peekable();
        for (i, &c) in chars.iter().enumerate() {
            if letters.next_if(|&&w| w == c).is_some() {
                found.push(i);
            }
        }
        if letters.peek().is_none() {
            marked.extend(found);
        }
    }
    marked.sort_unstable();
    marked.dedup();
    marked
}

fn lower(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

fn is_word_start(prev: char, c: char) -> bool {
    (!prev.is_alphanumeric() && c.is_alphanumeric())
        || (prev.is_lowercase() && c.is_uppercase())
        || (prev.is_alphabetic() && c.is_numeric())
}

/// Best alignment of `query` in `field` (dynamic programming over query
/// position × text position), or `None` if it isn't a subsequence.
fn score_field(query: &[char], field: &Field) -> Option<i32> {
    let text = &field.chars;
    let (m, n) = (query.len(), text.len());
    if m == 0 || m > n {
        return None;
    }
    const NONE: i32 = i32::MIN / 2;
    // prev[j]: best score with query[..i] matched and query[i - 1] at text[j].
    let mut prev = vec![NONE; n];
    for (j, &c) in text.iter().enumerate() {
        if c == query[0] {
            prev[j] = MATCH + field.bonus[j] - (j as i32 * GAP).min(LEADING_GAP_CAP);
        }
    }
    for &qc in &query[1..] {
        let mut cur = vec![NONE; n];
        // Best `prev[k] + k * GAP` for k < j - 1, so a gap from k to j costs
        // (j - k - 1) * GAP.
        let mut best_before = NONE;
        for j in 1..n {
            if j >= 2 {
                best_before = best_before.max(prev[j - 2] + (j as i32 - 2) * GAP);
            }
            if text[j] != qc {
                continue;
            }
            let consecutive = prev[j - 1] + CONSECUTIVE;
            let gapped = best_before - (j as i32 - 1) * GAP;
            let best = consecutive.max(gapped);
            if best > NONE / 2 {
                cur[j] = best + MATCH + field.bonus[j];
            }
        }
        prev = cur;
    }
    prev.into_iter().filter(|&s| s > NONE / 2).max()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score(text: &str, query: &str) -> Option<f64> {
        Haystack::new([(text, 1.0)]).score(query)
    }

    fn rank<'a>(texts: &[&'a str], query: &str) -> Vec<&'a str> {
        let mut scored: Vec<(f64, &str)> = texts
            .iter()
            .filter_map(|t| score(t, query).map(|s| (s, *t)))
            .collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        scored.into_iter().map(|(_, t)| t).collect()
    }

    #[test]
    fn requires_characters_in_order() {
        assert!(score("Firefox", "ffx").is_some());
        assert!(score("Firefox", "xff").is_none());
        assert!(score("Vim", "vimm").is_none());
        assert_eq!(score("Vim", "  "), Some(0.0));
    }

    #[test]
    fn prefers_word_starts_and_runs() {
        assert_eq!(
            rank(
                &["Calculator", "LibreOffice Calc", "Disk Usage Analyzer"],
                "calc"
            ),
            ["Calculator", "LibreOffice Calc"]
        );
        assert_eq!(
            rank(&["LibreOffice Draw", "LibreOffice Writer", "Weather"], "lw"),
            ["LibreOffice Writer", "LibreOffice Draw"]
        );
        // Matching at the start beats matching inside a word.
        assert_eq!(
            rank(&["Steam", "Text Editor"], "te"),
            ["Text Editor", "Steam"]
        );
        // camelCase and digits start words too.
        assert!(score("GitKraken", "k").unwrap() > score("Gitkraken", "k").unwrap());
        assert!(score("Python 3.12", "3").unwrap() > score("Python3x", "x").unwrap());
    }

    #[test]
    fn highlights_runs_then_letters() {
        assert_eq!(highlight("Firefox", "fire", true), [0, 1, 2, 3]);
        // A run at a word start beats an earlier one inside a word.
        assert_eq!(highlight("Kite Iterm", "ite", true), [5, 6, 7]);
        assert_eq!(highlight("LibreOffice Writer", "lw", true), [0, 12]);
        assert_eq!(highlight("Firefox", "FF", true), [0, 4]);
        assert_eq!(
            highlight("Firefox", "fire fox", true),
            [0, 1, 2, 3, 4, 5, 6]
        );
        // Words matching another field, and empty queries, mark nothing.
        assert_eq!(highlight("Firefox", "browser fox", true), [4, 5, 6]);
        assert!(highlight("Firefox", "", true).is_empty());
        assert!(highlight("Vim", "vimm", true).is_empty());
        // Runs only: scattered letters mark nothing.
        assert!(highlight("Formula Editor", "fi", false).is_empty());
        assert_eq!(highlight("LibreOffice", "fi", false), [7, 8]);
    }

    #[test]
    fn every_word_must_match_some_field() {
        let h = Haystack::new([("LibreOffice Writer", 1.0), ("Word Processor", 0.7)]);
        assert!(h.score("libre writer").is_some());
        assert!(h.score("libre word").is_some());
        assert!(h.score("libre spreadsheet").is_none());
        // A weaker field scores less than the same match in the name.
        let name = Haystack::new([("Files", 1.0)]).score("files").unwrap();
        let keyword = Haystack::new([("Nautilus", 1.0), ("Files", 0.6)])
            .score("files")
            .unwrap();
        assert!(name > keyword);
    }
}
