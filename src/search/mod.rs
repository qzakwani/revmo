//! Matching what's typed in the search box: fuzzy for apps, plain
//! substrings for menu lines.

mod fuzzy;
pub mod lines;

pub use fuzzy::{Haystack, highlight};
