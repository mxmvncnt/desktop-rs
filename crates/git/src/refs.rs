//! Branches: listing via libgit2, checkout, create, rename, delete.

use crate::{Repository, Result};

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
        let repo = self.repo()?;
        let mut branches = Vec::new();
        for branch in repo.branches(None)? {
            let (branch, branch_type) = branch?;
            let reference = branch.get();
            // Skip symbolic `origin/HEAD`.
            if reference.symbolic_target()?.is_some() {
                continue;
            }
            let (Some(name), Ok(tip)) = (branch.name()?, reference.peel_to_commit()) else { continue };
            let kind = if branch_type == git2::BranchType::Local { BranchKind::Local } else { BranchKind::Remote };
            let upstream = reference.name().ok().filter(|_| kind == BranchKind::Local).and_then(|r| upstream_ref(&repo, r));
            branches.push(Branch {
                name: name.to_string(),
                kind,
                is_head: branch.is_head(),
                upstream: upstream.and_then(|u| u.strip_prefix("refs/remotes/").map(String::from)),
                tip_time: tip.committer().when().seconds(),
            });
        }
        branches.sort_by_key(|b| (b.kind == BranchKind::Remote, std::cmp::Reverse(b.tip_time)));
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
            BranchKind::Local => self.git().args(["checkout", "-q", &branch.name]).run()?,
            BranchKind::Remote => {
                let local = branch
                    .name
                    .split_once('/')
                    .map(|(_, rest)| rest)
                    .unwrap_or(&branch.name);
                self.git()
                    .args(["checkout", "-q", "-b", local, "--track", &branch.name])
                    .run()?
            }
        };
        Ok(())
    }

    pub fn delete_branch(&self, name: &str, force: bool) -> Result<()> {
        let flag = if force { "-D" } else { "-d" };
        self.git().args(["branch", flag, name]).exec()
    }
}

/// Full ref name of the upstream configured for the local branch `refname`
/// (`refs/remotes/origin/main`), even when that ref no longer exists.
pub(crate) fn upstream_ref(repo: &git2::Repository, refname: &str) -> Option<String> {
    repo.branch_upstream_name(refname).ok()?.as_str().ok().map(String::from)
}
