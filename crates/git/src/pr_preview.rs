//! Pull-request preview: the merge-base comparison of a head branch against a
//! base branch, and the hosting provider's "new pull request" URL.

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use crate::diff::{Diff, DiffOptions, blob_sides};
use crate::tree_diff::tree_changes;
use crate::remote::{Host, host_of, split_remote};
use crate::{FileChange, GitError, Repository, Result};

/// What a PR from `head` into `base` would contain (`git diff base...head`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    /// Common ancestor of base and head: the left side of every file diff.
    pub merge_base: String,
    /// Commits on head that are not on base (`base..head`).
    pub ahead: u32,
    /// Commits on base that head doesn't have yet (`head..base`).
    pub behind: u32,
    pub files: Vec<FileChange>,
}

impl Repository {
    /// Branch a PR most likely targets: the remote's default (`origin/HEAD`),
    /// else the first of main / master / develop that exists.
    pub fn default_base_branch(&self) -> Option<String> {
        let repo = self.repo().ok()?;
        let origin_head = repo.find_reference("refs/remotes/origin/HEAD").ok();
        if let Some(target) = origin_head.as_ref().and_then(|r| r.symbolic_target().ok().flatten()) {
            return target.strip_prefix("refs/remotes/").map(String::from);
        }
        ["origin/main", "origin/master", "main", "master", "origin/develop", "develop"]
            .into_iter()
            .find(|n| {
                repo.find_reference(&format!("refs/remotes/{n}")).is_ok() || repo.find_reference(&format!("refs/heads/{n}")).is_ok()
            })
            .map(String::from)
    }

    /// Commits, behind count and file list of `base...head`.
    pub fn compare(&self, base: &str, head: &str) -> Result<Comparison> {
        let repo = self.repo()?;
        let base_commit = repo.revparse_single(base)?.peel_to_commit()?;
        let head_commit = repo.revparse_single(head)?.peel_to_commit()?;
        let merge_base = repo
            .merge_base(base_commit.id(), head_commit.id())
            .map_err(|_| GitError::Io(format!("{head} and {base} have no common history")))?;
        let (ahead, behind) = repo.graph_ahead_behind(head_commit.id(), base_commit.id())?;
        let files = tree_changes(&repo, Some(&repo.find_commit(merge_base)?.tree()?), &head_commit.tree()?)?;
        Ok(Comparison { merge_base: merge_base.to_string(), ahead: ahead as u32, behind: behind as u32, files })
    }

    /// Diff of one file of a [`Comparison`] (`merge_base` -> `head`).
    pub fn compare_file_diff(&self, merge_base: &str, head: &str, file: &FileChange, opts: DiffOptions) -> Result<Diff> {
        let (old, new) = blob_sides(merge_base, head, file);
        self.diff_sides(&old, &new, opts)
    }
}

/// Browser URL that opens a new pull / merge request. `base` and `head` are
/// branch names on the remote (no `origin/` prefix). `None` for unknown hosts.
pub fn compare_url(remote: &str, base: &str, head: &str) -> Option<String> {
    let (host, path) = split_remote(remote)?;
    Some(match host_of(remote)? {
        Host::GitHub => format!("https://{host}/{path}/compare/{}...{}?expand=1", enc(base, PATH), enc(head, PATH)),
        Host::GitLab => format!(
            "https://{host}/{path}/-/merge_requests/new?merge_request%5Bsource_branch%5D={}&merge_request%5Btarget_branch%5D={}",
            enc(head, COMPONENT),
            enc(base, COMPONENT)
        ),
        Host::Bitbucket => {
            format!("https://{host}/{path}/pull-requests/new?source={}&dest={}", enc(head, COMPONENT), enc(base, COMPONENT))
        }
    })
}

/// Unreserved characters stay as they are.
const COMPONENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~');
/// Like [`COMPONENT`], but keeps `/` for use in a URL path.
const PATH: &AsciiSet = &COMPONENT.remove(b'/');

fn enc(s: &str, set: &'static AsciiSet) -> String {
    utf8_percent_encode(s, set).to_string()
}

#[cfg(test)]
#[path = "tests/pr_preview.test.rs"]
mod tests;
