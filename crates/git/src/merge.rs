//! Merge / squash / rebase into the current branch, and conflict resolution.

use crate::progress::OpOutcome;
use crate::{message, GitError, Repository, Result};

/// What merging (or rebasing onto) `branch` would do, computed without
/// touching the working tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergePreview {
    pub branch: String,
    /// Commits reachable from `branch` but not from HEAD (`HEAD..branch`).
    pub ahead: u32,
    /// Commits reachable from HEAD but not from `branch` (`branch..HEAD`).
    pub behind: u32,
    /// Files that would conflict (only checked when both sides diverged).
    pub conflicts: Vec<String>,
}

/// Default subject of a squash-merge commit.
pub fn default_squash_summary(branch: &str, current: &str) -> String {
    format!("Squash merge {branch} into {current}")
}

impl Repository {
    /// Number of commits `branch` has that HEAD lacks (`HEAD..branch`).
    pub fn commits_ahead(&self, branch: &str) -> Result<u32> {
        let repo = self.repo()?;
        let head = repo.head()?.peel_to_commit()?.id();
        let (ahead, _) = repo.graph_ahead_behind(repo.revparse_single(branch)?.peel_to_commit()?.id(), head)?;
        Ok(ahead as u32)
    }

    pub fn merge_preview(&self, branch: &str) -> Result<MergePreview> {
        let repo = self.repo()?;
        let head = repo.head()?.peel_to_commit()?;
        let other = repo.revparse_single(branch)?.peel_to_commit()?;
        let (ahead, behind) = repo.graph_ahead_behind(other.id(), head.id())?;
        let mut conflicts = Vec::new();
        if ahead > 0 && behind > 0 {
            for conflict in repo.merge_commits(&head, &other, None)?.conflicts()? {
                let conflict = conflict?;
                if let Some(entry) = conflict.our.or(conflict.their).or(conflict.ancestor) {
                    conflicts.push(String::from_utf8_lossy(&entry.path).into_owned());
                }
            }
        }
        Ok(MergePreview { branch: branch.to_string(), ahead: ahead as u32, behind: behind as u32, conflicts })
    }

    fn current_branch_name(&self) -> String {
        self.git()
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .run()
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "HEAD".into())
    }

    /// Regular merge (`git merge --no-edit`). Conflicts leave the merge paused.
    pub fn merge(&self, branch: &str) -> Result<OpOutcome> {
        if self.commits_ahead(branch)? == 0 {
            return Ok(OpOutcome::UpToDate);
        }
        let res = self.git().no_editor().args(["merge", "--no-edit", branch]).run();
        self.outcome_of(res)
    }

    /// `git merge --squash` followed by one commit. `summary` is the commit
    /// subject (blank or `None`: "Squash merge <branch> into <current>"); the
    /// body lists the squashed commit subjects.
    /// Conflicts leave the index unmerged (reported as `InProgress::Squash`).
    pub fn merge_squash(&self, branch: &str, summary: Option<&str>) -> Result<OpOutcome> {
        if self.commits_ahead(branch)? == 0 {
            return Ok(OpOutcome::UpToDate);
        }
        let res = self.git().no_editor().args(["merge", "--squash", branch]).run();
        if self.outcome_of(res)? == OpOutcome::Conflicts {
            return Ok(OpOutcome::Conflicts);
        }
        // Nothing staged (e.g. the branch only reverted its own changes).
        if self.git().args(["diff", "--cached", "--quiet"]).run().is_ok() {
            return Ok(OpOutcome::UpToDate);
        }
        let subjects = self
            .git()
            .args(["log", "--reverse", "--format=* %s", &format!("HEAD..{branch}")])
            .run()
            .unwrap_or_default();
        let subject = match summary.map(str::trim) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => default_squash_summary(branch, &self.current_branch_name()),
        };
        self.git().args(["commit", "-q", "-F", "-"]).stdin(message(&subject, &subjects)).exec()?;
        Ok(OpOutcome::Done)
    }

    /// `git rebase <onto>` of the current branch. Conflicts leave the rebase paused.
    pub fn rebase(&self, onto: &str) -> Result<OpOutcome> {
        if self.commits_ahead(onto)? == 0 {
            return Ok(OpOutcome::UpToDate);
        }
        let res = self.git().no_editor().args(["rebase", onto]).run();
        self.outcome_of(res)
    }

    /// Commit during a merge-like operation (merge, squash, cherry-pick,
    /// revert): stage `paths` WITHOUT resetting the index (that would lose the
    /// merge result), then commit. git refuses while conflicts remain.
    pub fn commit_merge(&self, summary: &str, description: &str, paths: &[String]) -> Result<()> {
        if summary.trim().is_empty() {
            return Err(GitError::Invalid("commit summary is empty".into()));
        }
        self.stage(paths, &[])?;
        self.git()
            .no_editor()
            .args(["commit", "-q", "-F", "-"])
            .stdin(message(summary, description))
            .exec()
    }

    // ---- conflict resolution --------------------------------------------

    /// Resolve `path` with our side (`ours`) or theirs, then stage it. If that
    /// side deleted the file, the file is removed instead.
    pub fn resolve_with(&self, path: &str, theirs: bool) -> Result<()> {
        let side = if theirs { "--theirs" } else { "--ours" };
        match self.git().args(["checkout", side, "--", path]).run() {
            Ok(_) => {
                self.git().args(["add", "-A", "--", path]).run()?;
            }
            Err(GitError::Failed { stderr, .. }) if stderr.contains("does not have") => {
                self.git().args(["rm", "-q", "--", path]).run()?;
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    /// Mark `path` resolved with whatever is in the working tree.
    pub fn mark_resolved(&self, path: &str) -> Result<()> {
        self.git().args(["add", "-A", "--", path]).exec()
    }
}
