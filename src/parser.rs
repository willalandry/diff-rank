use std::collections::HashMap;
use std::io::{BufRead, Result};

/// Added/removed line counts for one file in a diff.
pub struct FileStat {
    pub path: String,
    pub added: u64,
    pub removed: u64,
}

impl FileStat {
    pub fn total(&self) -> u64 {
        self.added + self.removed
    }
}

/// Reads a unified diff line by line and returns per-file change counts,
/// ranked from most changed to least.
///
/// Only the running counters live in memory, not the diff text itself, so
/// this works the same whether the input is a ten-line patch or a
/// multi-gigabyte one piped in from `git log -p`.
pub fn rank<R: BufRead>(reader: R) -> Result<Vec<FileStat>> {
    let mut order: Vec<String> = Vec::new();
    let mut counts: HashMap<String, (u64, u64)> = HashMap::new();
    let mut current: Option<String> = None;
    // The path from the "--- a/..." line, held until we see the matching
    // "+++" so a deleted file (whose "+++" side is /dev/null) still has
    // somewhere to pull its path from.
    let mut old_path: Option<String> = None;

    for line in reader.lines() {
        let line = line?;

        if line.starts_with("diff --git") {
            // A new file section is starting; forget the old one so stray
            // lines before the next "+++" header aren't miscounted.
            current = None;
            old_path = None;
            continue;
        }

        if let Some(rest) = line.strip_prefix("--- ") {
            old_path = rest.strip_prefix("a/").map(|p| p.to_string());
            continue;
        }

        if let Some(rest) = line.strip_prefix("+++ ") {
            current = if rest == "/dev/null" {
                old_path.take()
            } else {
                rest.strip_prefix("b/").map(|p| p.to_string())
            };
            if let Some(path) = &current {
                if !counts.contains_key(path) {
                    order.push(path.clone());
                    counts.insert(path.clone(), (0, 0));
                }
            }
            continue;
        }

        if line.starts_with("@@") {
            continue;
        }

        if let Some(path) = &current {
            if line.starts_with('+') {
                counts.get_mut(path).unwrap().0 += 1;
            } else if line.starts_with('-') {
                counts.get_mut(path).unwrap().1 += 1;
            }
        }
    }

    let mut stats: Vec<FileStat> = order
        .into_iter()
        .map(|path| {
            let (added, removed) = counts[&path];
            FileStat { path, added, removed }
        })
        .collect();

    stats.sort_by(|a, b| b.total().cmp(&a.total()));
    Ok(stats)
}
