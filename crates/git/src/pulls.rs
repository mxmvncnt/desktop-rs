//! A pull request in the local repository: its head fetched from the remote (the forge keeps it under a ref of the base
//! repository, forks included), its commits and changes against the base branch, and checking it out as a branch.

use crate::diff_summary::LineStats;
use crate::log::Commit;
use crate::{FileChange, GitError, Repository, Result};

/// What a fetched pull request contains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullHead {
    pub head: String,
    /// Common ancestor of the base branch and the head: the left side of every file diff.
    pub merge_base: String,
    /// Oldest first, like the forges list them.
    pub commits: Vec<Commit>,
    pub files: Vec<FileChange>,
    pub stats: LineStats,
}

/// How to check out a pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullCheckout {
    /// The ref `fetch_pull` fetched the head into.
    pub pull_ref: String,
    /// Local branch to create or switch to.
    pub branch: String,
    /// The head branch on the tracking remote, which `branch` tracks; `None` for a fork (nothing to track here).
    pub upstream: Option<String>,
}

impl Repository {
    /// Fetch `pull_ref` from the tracking remote into the same ref here, and `base` (a branch name on the remote) into its
    /// remote-tracking branch, then compare them. `head_branch` is fetched too when given, so it can be checked out.
    pub fn fetch_pull(
        &self,
        pull_ref: &str,
        base: &str,
        head_branch: Option<&str>,
    ) -> Result<PullHead> {
        let remote = self
            .tracking_remote()
            .ok_or_else(|| GitError::Invalid("This repository has no remote.".into()))?;
        let base_ref = format!("refs/remotes/{remote}/{base}");
        let mut specs = vec![
            format!("+{pull_ref}:{pull_ref}"),
            format!("+refs/heads/{base}:{base_ref}"),
        ];
        if let Some(branch) = head_branch {
            specs.push(format!(
                "+refs/heads/{branch}:refs/remotes/{remote}/{branch}"
            ));
        }
        self.git()
            .args(["fetch", "--no-tags", "--no-write-fetch-head", &remote])
            .args(&specs)
            .network()
            .exec()?;
        self.pull_head(pull_ref, &base_ref)
    }

    /// Commits and changes of `head` against `base`, both already here.
    pub fn pull_head(&self, head: &str, base: &str) -> Result<PullHead> {
        let head = self.rev_parse(head)?;
        let merge_base = self
            .git()
            .args(["merge-base", base, &head])
            .run()?
            .trim()
            .to_string();
        let commits = self.log_commits(&["--reverse", &format!("{merge_base}..{head}")])?;
        let (files, stats) = self.range_changes(&merge_base, &head)?;
        Ok(PullHead {
            head,
            merge_base,
            commits,
            files,
            stats,
        })
    }

    /// Switch to `c.branch`, creating it at the fetched head. An existing branch moves to the head when that is a fast-forward,
    /// else it is only switched to, so local commits on it are kept.
    pub fn checkout_pull(&self, c: &PullCheckout) -> Result<()> {
        let local = format!("refs/heads/{}", c.branch);
        let upstream = match &c.upstream {
            Some(branch) => {
                let remote = self
                    .tracking_remote()
                    .ok_or_else(|| GitError::Invalid("This repository has no remote.".into()))?;
                Some(format!("{remote}/{branch}"))
            }
            None => None,
        };
        if self.rev_parse(&local).is_err() {
            let cmd = self.git().args(["checkout", "-q", "-b", &c.branch]);
            return match &upstream {
                Some(upstream) => cmd.args(["--track", upstream]),
                None => cmd.args(["--no-track", &c.pull_ref]),
            }
            .exec();
        }
        let fast_forward = self
            .git()
            .args(["merge-base", "--is-ancestor", &local, &c.pull_ref])
            .exec()
            .is_ok();
        if fast_forward {
            self.git()
                .args(["checkout", "-q", "-B", &c.branch, &c.pull_ref])
                .exec()?;
            if let Some(upstream) = &upstream {
                self.git()
                    .args(["branch", "-q", "--set-upstream-to", upstream, &c.branch])
                    .exec()?;
            }
            Ok(())
        } else {
            self.git().args(["checkout", "-q", &c.branch]).exec()
        }
    }

    fn rev_parse(&self, rev: &str) -> Result<String> {
        Ok(self
            .git()
            .args(["rev-parse", "--verify", "-q", &format!("{rev}^{{commit}}")])
            .run()?
            .trim()
            .to_string())
    }
}
