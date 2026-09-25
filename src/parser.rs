use std::collections::HashMap;
use std::io::{BufRead, Result};

/// Added/removed line counts for one file in a diff.
pub struct FileStat {
    pub path: String,
    pub added: u64,
    pub removed: u64,
    pub is_binary: bool,
}

impl FileStat {
    pub fn total(&self) -> u64 {
        self.added + self.removed
    }
}

/// (added, removed, is_binary) while a file section is being accumulated.
type Counts = (u64, u64, bool);

fn register(order: &mut Vec<String>, counts: &mut HashMap<String, Counts>, path: &str) {
    if !counts.contains_key(path) {
        order.push(path.to_string());
        counts.insert(path.to_string(), (0, 0, false));
    }
}

/// Pulls the "b/..." path out of a `diff --git a/x b/y` header line. This is
/// the only place a path is available for binary sections, which have no
/// "--- "/"+++ " headers of their own. Splits on the last " b/" the same way
/// git itself does, which can misparse paths that contain " b/" verbatim,
/// but that's an ambiguity in the diff format, not something this tool can
/// resolve.
fn parse_git_header_new_path(rest: &str) -> Option<String> {
    let idx = rest.rfind(" b/")?;
    Some(rest[idx + " b/".len()..].to_string())
}

fn is_binary_marker(line: &str) -> bool {
    line == "GIT binary patch" || (line.starts_with("Binary files ") && line.ends_with(" differ"))
}

/// Reads a unified diff line by line and returns per-file change counts,
/// ranked from most changed to least.
///
/// Only the running counters live in memory, not the diff text itself, so
/// this works the same whether the input is a ten-line patch or a
/// multi-gigabyte one piped in from `git log -p`.
pub fn rank<R: BufRead>(reader: R) -> Result<Vec<FileStat>> {
    let mut order: Vec<String> = Vec::new();
    let mut counts: HashMap<String, Counts> = HashMap::new();
    let mut current: Option<String> = None;
    // The path from the "--- a/..." line, held until we see the matching
    // "+++" so a deleted file (whose "+++" side is /dev/null) still has
    // somewhere to pull its path from.
    let mut old_path: Option<String> = None;
    // The "b/..." path parsed from the most recent "diff --git" header, kept
    // around in case this section turns out to be binary and so never gets
    // a "+++" line to set `current` from.
    let mut header_path: Option<String> = None;

    for line in reader.lines() {
        let line = line?;

        if let Some(rest) = line.strip_prefix("diff --git ") {
            // A new file section is starting; forget the old one so stray
            // lines before the next "+++" header aren't miscounted.
            current = None;
            old_path = None;
            header_path = parse_git_header_new_path(rest);
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
                register(&mut order, &mut counts, path);
            }
            continue;
        }

        if line.starts_with("@@") {
            continue;
        }

        if is_binary_marker(&line) {
            if current.is_none() {
                current = header_path.take();
            }
            if let Some(path) = &current {
                register(&mut order, &mut counts, path);
                counts.get_mut(path).unwrap().2 = true;
            }
            continue;
        }

        if let Some(path) = &current {
            let entry = counts.get_mut(path).unwrap();
            if entry.2 {
                // Binary section: the body is an encoded blob, not text, so
                // its lines don't count as added/removed even if they start
                // with '+' or '-'.
                continue;
            }
            if line.starts_with('+') {
                entry.0 += 1;
            } else if line.starts_with('-') {
                entry.1 += 1;
            }
        }
    }

    let mut stats: Vec<FileStat> = order
        .into_iter()
        .map(|path| {
            let (added, removed, is_binary) = counts[&path];
            FileStat {
                path,
                added,
                removed,
                is_binary,
            }
        })
        .collect();

    stats.sort_by(|a, b| b.total().cmp(&a.total()));
    Ok(stats)
}
