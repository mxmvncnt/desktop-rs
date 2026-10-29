//! Branches: listing via `git for-each-ref`, checkout, create, rename, delete.

use crate::{GitError, Repository, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BranchKind {
    Local,
    Remote,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Branch {
    /// Short name: `main` or `origin/main`.
    pub name: String,
    pub kind: BranchKind,
    pub is_head: bool,
    pub upstream: Option<String>,
    /// Committer date of the tip, unix seconds (for "recent" sorting).
    pub tip_time: i64,
}

impl Repository {
    /// Local branches first (most recently committed first), then remotes.
    pub fn branches(&self) -> Result<Vec<Branch>> {
        let out = self
            .git()
            .args([
                "for-each-ref",
                "--sort=-committerdate",
                "--format=%(refname)%00%(refname:short)%00%(HEAD)%00%(upstream:short)%00%(committerdate:unix)",
                "refs/heads",
                "refs/remotes",
            ])
            .run()?;
        let mut branches = Vec::new();
        for line in out.lines() {
            let [refname, name, head, upstream, time] = line.split('\0').collect::<Vec<_>>()[..] else {
                return Err(GitError::Parse(line.into()));
            };
            let kind = if refname.starts_with("refs/heads/") { BranchKind::Local } else { BranchKind::Remote };
            // Skip symbolic `origin/HEAD`.
            if kind == BranchKind::Remote && refname.ends_with("/HEAD") {
                continue;
            }
            branches.push(Branch {
                name: name.into(),
                kind,
                is_head: head == "*",
                upstream: (!upstream.is_empty()).then(|| upstream.into()),
                tip_time: time.parse().unwrap_or(0),
            });
        }
        branches.sort_by_key(|b| b.kind == BranchKind::Remote);
        Ok(branches)
    }

    /// Create `name` at `base` (any branch or commit-ish; default HEAD) and switch to it.
    pub fn create_branch(&self, name: &str, base: Option<&str>) -> Result<()> {
        self.git()
            .args(["checkout", "-q", "-b", name])
            .args(base)
            .arg("--no-track")
            .exec()
    }

    pub fn rename_branch(&self, old: &str, new: &str) -> Result<()> {
        self.git().args(["branch", "-m", old, new]).exec()
    }

    /// Delete `branch` on `remote` (`git push <remote> --delete <branch>`).
    pub fn delete_remote_branch(&self, remote: &str, branch: &str) -> Result<()> {
        self.git().args(["push", remote, "--delete", branch]).network().exec()
    }

    /// Switch to an existing local branch, or create a tracking branch for a
    /// remote one (`origin/foo` → `foo`).
    pub fn checkout(&self, branch: &Branch) -> Result<()> {
        match branch.kind {
            BranchKind::Local => self.git().args(["checkout", "-q", &branch.name]).exec(),
            BranchKind::Remote => {
                let local = branch.name.split_once('/').map_or(&*branch.name, |(_, rest)| rest);
                self.git().args(["checkout", "-q", "-b", local, "--track", &branch.name]).exec()
            }
        }
    }

    pub fn delete_branch(&self, name: &str, force: bool) -> Result<()> {
        let flag = if force { "-D" } else { "-d" };
        self.git().args(["branch", flag, name]).exec()
    }
}
