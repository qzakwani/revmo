//! Launch history, used to rank apps by "frecency": how often and how
//! recently each one was launched.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// A launch's weight halves every this many days.
const HALF_LIFE_DAYS: f64 = 14.0;
const SECS_PER_DAY: f64 = 86_400.0;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Usage {
    entries: HashMap<String, Entry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Entry {
    /// Decayed launch count as of `last`.
    score: f64,
    /// Unix time of the last launch.
    last: u64,
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn decay(score: f64, from: u64, to: u64) -> f64 {
    let days = to.saturating_sub(from) as f64 / SECS_PER_DAY;
    score * 0.5_f64.powf(days / HALF_LIFE_DAYS)
}

impl Usage {
    /// Reads the usage file; a missing or unreadable file is empty history.
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .map(|s| Usage::parse(&s))
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        // To the file a symlink points to, so a dotfiles link stays a link.
        let target = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        crate::files::write_atomic(&target, self.serialize().as_bytes())
    }

    /// One `score<TAB>last<TAB>key` line per app.
    fn parse(contents: &str) -> Self {
        let entries = contents
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(3, '\t');
                let score = parts.next()?.parse().ok()?;
                let last = parts.next()?.parse().ok()?;
                let key = parts.next().filter(|k| !k.is_empty())?;
                Some((key.to_string(), Entry { score, last }))
            })
            .collect();
        Usage { entries }
    }

    fn serialize(&self) -> String {
        let mut keys: Vec<&String> = self.entries.keys().collect();
        keys.sort();
        keys.iter()
            .map(|k| {
                let e = self.entries[*k];
                format!("{:.4}\t{}\t{}\n", e.score, e.last, k)
            })
            .collect()
    }

    pub fn record(&mut self, key: &str, now: u64) {
        let entry = self.entries.entry(key.to_string()).or_insert(Entry {
            score: 0.0,
            last: now,
        });
        entry.score = decay(entry.score, entry.last, now) + 1.0;
        entry.last = entry.last.max(now);
    }

    /// Higher is more relevant; 0 for apps never launched.
    pub fn score(&self, key: &str, now: u64) -> f64 {
        self.entries
            .get(key)
            .map_or(0.0, |e| decay(e.score, e.last, now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u64 = 86_400;

    #[test]
    fn frequent_and_recent_rank_higher() {
        let t = 1_000 * DAY;
        let mut usage = Usage::default();
        for _ in 0..5 {
            usage.record("often.desktop", t);
        }
        usage.record("once.desktop", t);
        usage.record("old.desktop", t - 60 * DAY);

        assert!(usage.score("often.desktop", t) > usage.score("once.desktop", t));
        assert!(usage.score("once.desktop", t) > usage.score("old.desktop", t));
        assert_eq!(usage.score("never.desktop", t), 0.0);
        // Two weeks later a launch counts half.
        assert!((usage.score("once.desktop", t + 14 * DAY) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn round_trips_and_skips_bad_lines() {
        let mut usage = Usage::default();
        usage.record("a.desktop", 100);
        usage.record("b.desktop/new-window", 200);
        let parsed = Usage::parse(&format!("garbage\n1\tx\tc\n{}", usage.serialize()));
        assert_eq!(parsed, usage);

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nested/usage");
        usage.save(&path).unwrap();
        assert_eq!(Usage::load(&path), usage);
        assert_eq!(Usage::load(&tmp.path().join("missing")), Usage::default());
    }
}
