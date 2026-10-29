//! Working-directory status via `git status --porcelain=v2 -z --branch`.

use crate::{GitError, Repository, Result};

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
    /// The branch diverged from its upstream because local history was rewritten
    /// (the upstream tip used to be a tip of this branch): needs a force push, not a pull.
    pub rewritten: bool,
    pub files: Vec<FileChange>,
}

impl Repository {
    pub fn status(&self) -> Result<WorkingStatus> {
        let out = self
            .git()
            .args([
                "status",
                "--porcelain=v2",
                "-z",
                "--branch",
                "--untracked-files=all",
                "--ignore-submodules=dirty",
            ])
            .run_bytes()?;
        let mut st = parse_status(&out)?;
        // Diverged from the upstream, but the upstream tip is one of this
        // branch's earlier tips: history was rewritten.
        if let (Some(branch), Some(_)) = (&st.branch, &st.upstream)
            && st.ahead_behind.ahead > 0
            && st.ahead_behind.behind > 0
        {
            let tip = self.git().args(["rev-parse", "--verify", "@{upstream}"]).run()?;
            let reflog = self.git().args(["reflog", "show", "--format=%H"]).arg(format!("refs/heads/{branch}")).run()?;
            let tip = tip.trim();
            st.rewritten = reflog.lines().any(|sha| sha == tip);
        }
        Ok(st)
    }
}

fn status_from_xy(xy: &str) -> FileStatus {
    // Desktop shows the combined (index + worktree) state relative to HEAD.
    match xy.as_bytes() {
        [b'A', _] => FileStatus::New,
        [b'D', _] | [_, b'D'] => FileStatus::Deleted,
        [b'R', _] => FileStatus::Renamed,
        [b'C', _] => FileStatus::Copied,
        [b'T', _] | [_, b'T'] => FileStatus::TypeChanged,
        _ => FileStatus::Modified,
    }
}

pub(crate) fn parse_status(out: &[u8]) -> Result<WorkingStatus> {
    let mut st = WorkingStatus::default();
    let mut fields = out.split(|&b| b == 0).filter(|f| !f.is_empty()).map(String::from_utf8_lossy);
    while let Some(line) = fields.next() {
        let bad = || GitError::Parse(line.to_string());
        let Some((kind, rest)) = line.split_once(' ') else { return Err(bad()) };
        match kind {
            "#" => {
                let (key, value) = rest.split_once(' ').unwrap_or((rest, ""));
                match key {
                    "branch.oid" if value != "(initial)" => st.head_oid = Some(value.into()),
                    "branch.head" if value != "(detached)" => st.branch = Some(value.into()),
                    "branch.upstream" => st.upstream = Some(value.into()),
                    "branch.ab" => {
                        // +<ahead> -<behind>
                        let count = |s: &str, sign| s.strip_prefix(sign)?.parse().ok();
                        let (ahead, behind) = value.split_once(' ').ok_or_else(bad)?;
                        st.ahead_behind = AheadBehind {
                            ahead: count(ahead, '+').ok_or_else(bad)?,
                            behind: count(behind, '-').ok_or_else(bad)?,
                        };
                    }
                    _ => {}
                }
            }
            // 1 XY sub mH mI mW hH hI path
            "1" => {
                let [xy, _, _, _, _, _, _, path] = rest.splitn(8, ' ').collect::<Vec<_>>()[..] else { return Err(bad()) };
                st.files.push(FileChange { path: path.into(), old_path: None, status: status_from_xy(xy) });
            }
            // 2 XY sub mH mI mW hH hI Xscore path \0 origPath
            "2" => {
                let [xy, _, _, _, _, _, _, score, path] = rest.splitn(9, ' ').collect::<Vec<_>>()[..] else {
                    return Err(bad());
                };
                let status = match status_from_xy(xy) {
                    _ if score.starts_with('C') => FileStatus::Copied,
                    FileStatus::Modified => FileStatus::Renamed,
                    s => s,
                };
                let old_path = fields.next().map(|p| p.into_owned());
                st.files.push(FileChange { path: path.into(), old_path, status });
            }
            // u XY sub m1 m2 m3 mW h1 h2 h3 path
            "u" => {
                let Some(path) = rest.splitn(10, ' ').nth(9) else { return Err(bad()) };
                st.files.push(FileChange { path: path.into(), old_path: None, status: FileStatus::Conflicted });
            }
            "?" => st.files.push(FileChange { path: rest.into(), old_path: None, status: FileStatus::Untracked }),
            _ => {}
        }
    }
    // Conflicted files first (Desktop lists them in their own section on top).
    st.files.sort_by(|a, b| {
        (a.status != FileStatus::Conflicted, &a.path).cmp(&(b.status != FileStatus::Conflicted, &b.path))
    });
    Ok(st)
}

#[cfg(test)]
#[path = "tests/status.test.rs"]
mod tests;
