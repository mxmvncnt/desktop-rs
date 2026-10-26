//! Parser for `git ... --name-status -z` output.

use crate::{FileChange, FileStatus};

pub(crate) fn parse_name_status(out: &str) -> Vec<FileChange> {
    let mut fields = out.split('\0').filter(|s| !s.is_empty());
    let mut files = Vec::new();
    while let Some(code) = fields.next() {
        let code = code.trim_start_matches('\n');
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
