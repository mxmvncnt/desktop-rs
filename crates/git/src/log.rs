//! History: commit list and per-commit file lists.

use crate::diff_summary::{LineStats, parse_name_status, parse_shortstat};
use crate::{FileChange, Repository, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Commit {
    pub sha: String,
    pub short_sha: String,
    pub author_name: String,
    pub author_email: String,
    /// Author date, unix seconds.
    pub time: i64,
    pub summary: String,
    pub body: String,
    pub parents: Vec<String>,
    /// From `Co-authored-by:` trailers in the body.
    pub co_authors: Vec<CoAuthor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CoAuthor {
    pub name: String,
    pub email: String,
}

/// Parse `Co-authored-by: Name <email>` trailers (case-insensitive key).
pub fn parse_co_authors(body: &str) -> Vec<CoAuthor> {
    let mut out: Vec<CoAuthor> = Vec::new();
    for line in body.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        if !key.trim().eq_ignore_ascii_case("co-authored-by") {
            continue;
        }
        let value = value.trim();
        let (name, email) = match (value.rfind('<'), value.ends_with('>')) {
            (Some(i), true) => (value[..i].trim(), value[i + 1..value.len() - 1].trim()),
            _ => (value, ""),
        };
        let name = if name.is_empty() { email } else { name };
        if name.is_empty() || out.iter().any(|c| c.email == email && c.name == name) {
            continue;
        }
        out.push(CoAuthor { name: name.into(), email: email.into() });
    }
    out
}

const SEP: char = '\u{1f}';

impl Repository {
    /// `git log` with `args` (revisions, limits), parsed.
    pub(crate) fn log_commits(&self, args: &[&str]) -> Result<Vec<Commit>> {
        let out = self
            .git()
            .args(["log", "-z", "--format=%H%x1f%h%x1f%an%x1f%ae%x1f%at%x1f%P%x1f%s%x1f%b"])
            .args(args)
            .run()?;
        Ok(out
            .split('\0')
            .filter(|r| !r.is_empty())
            .filter_map(|record| {
                let [sha, short_sha, author_name, author_email, time, parents, summary, body] =
                    <[&str; 8]>::try_from(record.splitn(8, SEP).collect::<Vec<_>>()).ok()?;
                let body = body.trim_end();
                Some(Commit {
                    co_authors: parse_co_authors(body),
                    sha: sha.into(),
                    short_sha: short_sha.into(),
                    author_name: author_name.into(),
                    author_email: author_email.into(),
                    time: time.parse().unwrap_or(0),
                    parents: parents.split_whitespace().map(Into::into).collect(),
                    summary: summary.into(),
                    body: body.into(),
                })
            })
            .collect())
    }

    /// Commits reachable from HEAD, newest first. Paged with `skip`/`limit`
    /// so the history list can load lazily as the user scrolls.
    pub fn log(&self, skip: usize, limit: usize) -> Result<Vec<Commit>> {
        if !self.has_head() {
            return Ok(Vec::new());
        }
        self.log_commits(&[&format!("--skip={skip}"), &format!("--max-count={limit}"), "HEAD"])
    }

    /// Files changed by `sha` (against its first parent; root commits against
    /// the empty tree).
    pub fn commit_files(&self, sha: &str) -> Result<Vec<FileChange>> {
        let out = self
            .git()
            .args([
                "show",
                "--format=",
                "-z",
                "--name-status",
                "-M",
                "--diff-merges=first-parent",
                sha,
            ])
            .run()?;
        Ok(parse_name_status(&out))
    }

    /// Total added / deleted lines of `sha` (same base as [`Self::commit_files`]).
    pub fn commit_stats(&self, sha: &str) -> Result<LineStats> {
        let out = self
            .git()
            .args(["show", "--format=", "--shortstat", "-M", "--diff-merges=first-parent", sha])
            .run()?;
        Ok(parse_shortstat(&out))
    }
}

#[cfg(test)]
#[path = "tests/log.test.rs"]
mod tests;
