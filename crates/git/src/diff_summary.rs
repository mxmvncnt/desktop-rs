//! Parsers for `git ... --name-status -z` and `--shortstat` output, and the
//! file list + line totals of a revision range.

use crate::{FileChange, FileStatus, Repository, Result};

/// Total added / deleted lines of a set of changes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LineStats {
    pub added: u32,
    pub deleted: u32,
}

/// `" 3 files changed, 12 insertions(+), 1 deletion(-)"` (empty when nothing changed).
/// Binary files count for no lines.
pub(crate) fn parse_shortstat(out: &str) -> LineStats {
    let mut stats = LineStats::default();
    for part in out.split(',') {
        let mut words = part.split_whitespace();
        let n = words.next().and_then(|n| n.parse().ok()).unwrap_or(0);
        match words.next() {
            Some(w) if w.starts_with("insertion") => stats.added = n,
            Some(w) if w.starts_with("deletion") => stats.deleted = n,
            _ => {}
        }
    }
    stats
}

/// `--name-status -z` output: `<code>\0<path>\0`, or `<code>\0<old>\0<new>\0` for renames and copies.
pub(crate) fn parse_name_status(out: &str) -> Vec<FileChange> {
    let mut fields = out.split('\0').filter(|s| !s.is_empty());
    let mut files = Vec::new();
    while let Some(code) = fields.next() {
        let status = match code.as_bytes().first() {
            Some(b'A') => FileStatus::New,
            Some(b'D') => FileStatus::Deleted,
            Some(b'R') => FileStatus::Renamed,
            Some(b'C') => FileStatus::Copied,
            Some(b'T') => FileStatus::TypeChanged,
            Some(b'U') => FileStatus::Conflicted,
            _ => FileStatus::Modified,
        };
        let first = fields.next().unwrap_or_default().to_string();
        let (path, old_path) = if matches!(status, FileStatus::Renamed | FileStatus::Copied) {
            (fields.next().unwrap_or_default().to_string(), Some(first))
        } else {
            (first, None)
        };
        files.push(FileChange { path, old_path, status });
    }
    files
}

impl Repository {
    /// Net file changes and line totals from `from` to `to` (renames detected).
    pub(crate) fn range_changes(&self, from: &str, to: &str) -> Result<(Vec<FileChange>, LineStats)> {
        let diff = |format: &[&str]| {
            self.git().args(["diff", "-M", "--no-ext-diff"]).args(format).args([from, to]).run()
        };
        Ok((parse_name_status(&diff(&["--name-status", "-z"])?), parse_shortstat(&diff(&["--shortstat"])?)))
    }
}

#[cfg(test)]
#[path = "tests/diff_summary.test.rs"]
mod tests;
