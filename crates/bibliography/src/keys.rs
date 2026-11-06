use crate::normalize_text;
use hayagriva::Entry;
use std::collections::BTreeSet;

fn slug(value: &str) -> String {
    normalize_text(value)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// A suggested name, never identity evidence or a request to rename another notice.
pub fn suggest_key(entry: &Entry, occupied: &BTreeSet<String>) -> String {
    let author = entry
        .authors()
        .or_else(|| entry.editors())
        .and_then(|p| p.first())
        .map(|p| slug(&p.name))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "anonymous".into());
    let title = entry
        .title()
        .map(|t| slug(&t.value.to_string()))
        .and_then(|s| s.split('-').next().map(ToOwned::to_owned))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "untitled".into());
    let date = entry
        .date_any()
        .map(|d| d.year.to_string())
        .unwrap_or_else(|| "nd".into());
    let base = format!("{author}_{title}_{date}")
        .chars()
        .take(96)
        .collect::<String>();
    if !occupied.contains(&base) {
        return base;
    }
    for number in 1usize.. {
        let candidate = format!("{base}_{number}");
        if !occupied.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!("A finite library cannot occupy every suffix")
}
