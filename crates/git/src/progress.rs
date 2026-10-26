//! Detection (and generic continue/abort) of a multi-step operation that is
//! paused in the git directory: merge, squash-merge, rebase, cherry-pick, revert.
//!
//! Detection reads the git directory through libgit2, so the UI loads it on
//! every status refresh. Other features (revert,
//! cherry-pick, squash from History) need no extra code to get the banner,
//! the conflict list and Continue/Abort: they only need to start their `git`
//! command and let the operation pause.


use git2::RepositoryState;

use crate::{Repository, Result};

/// What is paused in this repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InProgress {
    /// `MERGE_HEAD` exists. `branch` comes from `MERGE_MSG` ("Merge branch 'x'").
    Merge { branch: Option<String>, head: String },
    /// `git merge --squash` stopped on conflicts (`SQUASH_MSG` + unmerged paths).
    Squash,
    /// `rebase-merge/` or `rebase-apply/`.
    Rebase {
        branch: Option<String>,
        onto: Option<String>,
        /// 1-based number of the commit being applied.
        step: u32,
        total: u32,
    },
    /// `CHERRY_PICK_HEAD`.
    CherryPick { sha: String, subject: String },
    /// `REVERT_HEAD`.
    Revert { sha: String, subject: String },
}

impl InProgress {
    /// Short verb for banners: "Merging", "Rebasing", ...
    pub fn verb(&self) -> &'static str {
        match self {
            Self::Merge { .. } => "Merging",
            Self::Squash => "Squash merging",
            Self::Rebase { .. } => "Rebasing",
            Self::CherryPick { .. } => "Cherry-picking",
            Self::Revert { .. } => "Reverting",
        }
    }

    /// Whether committing from the commit box finishes the operation
    /// (merge-like) as opposed to `--continue` only (rebase).
    pub fn commit_finishes(&self) -> bool {
        !matches!(self, Self::Rebase { .. })
    }
}

/// Result of an operation that can pause on conflicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpOutcome {
    /// Completed (a commit was created, or the rebase finished).
    Done,
    /// Nothing to do.
    UpToDate,
    /// Paused: resolve conflicts, then continue (see [`InProgress`]).
    Conflicts,
}

/// `Merge branch 'feat' into main` / `Merge remote-tracking branch 'origin/x'` -> `feat`.
fn merge_branch_from_msg(msg: &str) -> Option<String> {
    let first = msg.lines().next()?;
    let start = first.find('\'')? + 1;
    let end = first[start..].find('\'')? + start;
    Some(first[start..end].to_string())
}

/// Branch name if one points at `oid`, else the short sha.
fn describe_commit(repo: &git2::Repository, oid: git2::Oid) -> String {
    let branch = repo.branches(None).ok().and_then(|branches| {
        branches.flatten().find_map(|(b, _)| {
            (b.get().target() == Some(oid)).then(|| b.name().ok().flatten().map(String::from)).flatten()
        })
    });
    branch.unwrap_or_else(|| oid.to_string().chars().take(7).collect())
}

impl Repository {
    /// The paused operation, if any. Cheap; call from a background task.
    pub fn in_progress(&self) -> Option<InProgress> {
        self.paused_operation().ok().flatten()
    }

    fn paused_operation(&self) -> Result<Option<InProgress>> {
        let repo = self.repo()?;
        Ok(Some(match repo.state() {
            // libgit2's `open_rebase` rejects the `interactive` marker that git's own
            // merge backend writes, so the step files are read directly.
            RepositoryState::Rebase | RepositoryState::RebaseInteractive | RepositoryState::RebaseMerge => {
                let dir = repo.path().join("rebase-merge");
                let read = |file: &str| std::fs::read_to_string(dir.join(file)).ok().map(|s| s.trim().to_string());
                InProgress::Rebase {
                    branch: read("head-name")
                        .map(|h| h.strip_prefix("refs/heads/").unwrap_or(&h).to_string())
                        .filter(|h| h != "detached HEAD"),
                    onto: read("onto").and_then(|sha| git2::Oid::from_str(&sha).ok()).map(|oid| describe_commit(&repo, oid)),
                    step: read("msgnum").and_then(|s| s.parse().ok()).unwrap_or(0),
                    total: read("end").and_then(|s| s.parse().ok()).unwrap_or(0),
                }
            }
            RepositoryState::Merge => InProgress::Merge {
                branch: repo.message().ok().and_then(|m| merge_branch_from_msg(&m)),
                head: repo.revparse_single("MERGE_HEAD")?.id().to_string(),
            },
            state @ (RepositoryState::CherryPick
            | RepositoryState::CherryPickSequence
            | RepositoryState::Revert
            | RepositoryState::RevertSequence) => {
                let revert = matches!(state, RepositoryState::Revert | RepositoryState::RevertSequence);
                let commit = repo.revparse_single(if revert { "REVERT_HEAD" } else { "CHERRY_PICK_HEAD" })?.peel_to_commit()?;
                let sha = commit.id().to_string().chars().take(7).collect();
                let subject = commit.summary().ok().flatten().unwrap_or_default().to_string();
                if revert { InProgress::Revert { sha, subject } } else { InProgress::CherryPick { sha, subject } }
            }
            _ if repo.path().join("SQUASH_MSG").is_file() && repo.index()?.has_conflicts() => InProgress::Squash,
            _ => return Ok(None),
        }))
    }

    /// Whether the index has unmerged (conflicted) entries.
    pub fn has_unmerged(&self) -> bool {
        self.repo().and_then(|r| Ok(r.index()?.has_conflicts())).unwrap_or(false)
    }

    /// The message git prepared for the operation (`MERGE_MSG`, else
    /// `SQUASH_MSG`) as (summary, description), comment lines removed.
    pub fn prepared_message(&self) -> Option<(String, String)> {
        let dir = self.git_dir();
        let raw = std::fs::read_to_string(dir.join("MERGE_MSG"))
            .or_else(|_| std::fs::read_to_string(dir.join("SQUASH_MSG")))
            .ok()?;
        let text: String = raw
            .lines()
            .filter(|l| !l.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let (summary, rest) = text.split_once('\n').unwrap_or((text, ""));
        Some((summary.trim().to_string(), rest.trim().to_string()))
    }

    /// Finish the paused operation (all conflicts must be resolved and staged).
    /// Returns [`OpOutcome::Conflicts`] when a rebase / multi-pick stops again.
    pub fn continue_operation(&self, op: &InProgress) -> Result<OpOutcome> {
        let res = match op {
            InProgress::Merge { .. } => self.git().no_editor().args(["merge", "--continue"]).run(),
            InProgress::Squash => self
                .git()
                .no_editor()
                .args(["commit", "-q", "--cleanup=strip", "-F"])
                .arg(self.git_dir().join("SQUASH_MSG"))
                .run(),
            InProgress::Rebase { .. } => self.git().no_editor().args(["rebase", "--continue"]).run(),
            InProgress::CherryPick { .. } => {
                self.git().no_editor().args(["cherry-pick", "--continue"]).run()
            }
            InProgress::Revert { .. } => self.git().no_editor().args(["revert", "--continue"]).run(),
        };
        self.outcome_of(res)
    }

    /// Abort the paused operation and restore the pre-operation state.
    pub fn abort_operation(&self, op: &InProgress) -> Result<()> {
        let args: &[&str] = match op {
            InProgress::Merge { .. } => &["merge", "--abort"],
            InProgress::Squash => &["reset", "-q", "--merge"],
            InProgress::Rebase { .. } => &["rebase", "--abort"],
            InProgress::CherryPick { .. } => &["cherry-pick", "--abort"],
            InProgress::Revert { .. } => &["revert", "--abort"],
        };
        self.git().no_editor().args(args.iter().copied()).exec()
    }

    /// Map a failed git command to `Conflicts` when it merely paused.
    pub(crate) fn outcome_of<T>(&self, res: Result<T>) -> Result<OpOutcome> {
        match res {
            Ok(_) => Ok(OpOutcome::Done),
            Err(e) => {
                if self.in_progress().is_some() && self.has_unmerged() {
                    Ok(OpOutcome::Conflicts)
                } else {
                    Err(e)
                }
            }
        }
    }
}

