//! File lists between two trees (commit, stash, pull-request comparison).

use git2::{Delta, DiffFindOptions, Tree};

use crate::{FileChange, FileStatus, Result};

/// Files changed from `old` (`None` = empty tree) to `new`, with renames detected.
pub(crate) fn tree_changes(repo: &git2::Repository, old: Option<&Tree>, new: &Tree) -> Result<Vec<FileChange>> {
    let mut diff = repo.diff_tree_to_tree(old, Some(new), None)?;
    diff.find_similar(Some(DiffFindOptions::new().renames(true)))?;
    Ok(diff
        .deltas()
        .filter_map(|d| {
            let status = match d.status() {
                Delta::Added => FileStatus::New,
                Delta::Deleted => FileStatus::Deleted,
                Delta::Renamed => FileStatus::Renamed,
                Delta::Copied => FileStatus::Copied,
                Delta::Typechange => FileStatus::TypeChanged,
                Delta::Conflicted => FileStatus::Conflicted,
                _ => FileStatus::Modified,
            };
            let path = |f: git2::DiffFile| f.path().map(|p| p.to_string_lossy().into_owned());
            let old_path = matches!(status, FileStatus::Renamed | FileStatus::Copied).then(|| path(d.old_file())).flatten();
            Some(FileChange { path: path(d.new_file())?, old_path, status })
        })
        .collect())
}
