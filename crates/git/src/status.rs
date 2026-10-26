//! Working-directory status via libgit2.

use git2::{Status, StatusOptions};

use crate::{Repository, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileStatus {
    New,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Conflicted,
    Untracked,
    TypeChanged,
}

impl FileStatus {
    /// Single-letter badge used in the file list.
    pub fn badge(self) -> &'static str {
        match self {
            Self::New | Self::Untracked => "+",
            Self::Modified | Self::TypeChanged => "•",
            Self::Deleted => "−",
            Self::Renamed | Self::Copied => "→",
            Self::Conflicted => "!",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileChange {
    /// Path relative to the repository root.
    pub path: String,
    /// Previous path for renames/copies.
    pub old_path: Option<String>,
    pub status: FileStatus,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AheadBehind {
    pub ahead: u32,
    pub behind: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkingStatus {
    /// Current branch name; `None` when HEAD is detached.
    pub branch: Option<String>,
    /// Commit HEAD points to; `None` before the first commit.
    pub head_oid: Option<String>,
    pub upstream: Option<String>,
    /// Only meaningful when `upstream` is set.
    pub ahead_behind: AheadBehind,
    pub files: Vec<FileChange>,
}

impl Repository {
    pub fn status(&self) -> Result<WorkingStatus> {
        let repo = self.repo()?;
        let mut st = WorkingStatus::default();
        let head = repo.find_reference("HEAD")?;
        st.branch = head.symbolic_target()?.and_then(|t| t.strip_prefix("refs/heads/")).map(String::from);
        st.head_oid = head.resolve().ok().and_then(|h| h.target()).map(|oid| oid.to_string());
        let upstream = st
            .branch
            .as_ref()
            .and_then(|b| repo.branch_upstream_name(&format!("refs/heads/{b}")).ok())
            .and_then(|n| n.as_str().ok().map(String::from));
        if let Some(upstream) = upstream {
            st.upstream = upstream.strip_prefix("refs/remotes/").map(String::from);
            if let (Some(local), Ok(remote)) = (&st.head_oid, repo.refname_to_id(&upstream)) {
                let (ahead, behind) = repo.graph_ahead_behind(local.parse()?, remote)?;
                st.ahead_behind = AheadBehind { ahead: ahead as u32, behind: behind as u32 };
            }
        }

        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .include_ignored(false)
            .recurse_untracked_dirs(true)
            .renames_head_to_index(true);
        for entry in repo.statuses(Some(&mut opts))?.iter() {
            let Ok(path) = entry.path() else { continue };
            let flags = entry.status();
            let status = if flags.is_conflicted() {
                FileStatus::Conflicted
            } else if flags.contains(Status::INDEX_NEW) {
                FileStatus::New
            } else if flags.contains(Status::WT_NEW) {
                FileStatus::Untracked
            } else if flags.intersects(Status::INDEX_DELETED | Status::WT_DELETED) {
                FileStatus::Deleted
            } else if flags.contains(Status::INDEX_RENAMED) {
                FileStatus::Renamed
            } else if flags.intersects(Status::INDEX_TYPECHANGE | Status::WT_TYPECHANGE) {
                FileStatus::TypeChanged
            } else {
                FileStatus::Modified
            };
            let rename = entry.head_to_index().filter(|_| status == FileStatus::Renamed);
            let path_of = |f: git2::DiffFile| f.path().map(|p| p.to_string_lossy().into_owned());
            st.files.push(match rename {
                Some(d) => FileChange { path: path_of(d.new_file()).unwrap_or_default(), old_path: path_of(d.old_file()), status },
                None => FileChange { path: path.to_string(), old_path: None, status },
            });
        }
        // Conflicted files first (Desktop lists them in their own section on top).
        st.files.sort_by(|a, b| {
            (a.status != FileStatus::Conflicted, &a.path).cmp(&(b.status != FileStatus::Conflicted, &b.path))
        });
        Ok(st)
    }
}
