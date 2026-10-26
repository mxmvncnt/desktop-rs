//! Pull-request preview: the merge-base comparison of a head branch against a
//! base branch, and the hosting provider's "new pull request" URL.

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use crate::diff::{Diff, DiffOptions, blob_sides};
use crate::name_status::parse_name_status;
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
        if let Ok(out) = self.git().args(["symbolic-ref", "-q", "--short", "refs/remotes/origin/HEAD"]).run() {
            let name = out.trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
        ["origin/main", "origin/master", "main", "master", "origin/develop", "develop"]
            .into_iter()
            .find(|n| self.git().args(["rev-parse", "--verify", "-q"]).arg(format!("refs/remotes/{n}")).run().is_ok()
                || self.git().args(["rev-parse", "--verify", "-q"]).arg(format!("refs/heads/{n}")).run().is_ok())
            .map(String::from)
    }

    /// Commits, behind count and file list of `base...head`.
    pub fn compare(&self, base: &str, head: &str) -> Result<Comparison> {
        let merge_base = self
            .git()
            .args(["merge-base", base, head])
            .run()
            .map_err(|_| GitError::Io(format!("{head} and {base} have no common history")))?
            .trim()
            .to_string();
        let ahead = self.count(&format!("{base}..{head}"))?;
        let behind = self.count(&format!("{head}..{base}"))?;
        let out = self
            .git()
            .args(["diff", "--name-status", "-z", "-M", "--no-ext-diff"])
            .arg(&merge_base)
            .arg(head)
            .run()?;
        Ok(Comparison { merge_base, ahead, behind, files: parse_name_status(&out) })
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
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert_eq!(
            compare_url("git@github.com:o/r.git", "main", "feat/x").unwrap(),
            "https://github.com/o/r/compare/main...feat/x?expand=1"
        );
        assert!(compare_url("https://gitlab.com/o/r.git", "main", "a b").unwrap().ends_with("target_branch%5D=main"));
        assert!(compare_url("https://example.com/o/r.git", "main", "x").is_none());
    }
}
