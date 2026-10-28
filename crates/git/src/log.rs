//! History: commit list and per-commit file lists.

use crate::tree_diff::tree_changes;
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

/// `Co-authored-by: Name <email>` trailers of a commit message (case-insensitive key).
fn parse_co_authors(message: &str) -> Vec<CoAuthor> {
    let Ok(trailers) = git2::message_trailers_strs(message) else { return Vec::new() };
    let mut out: Vec<CoAuthor> = Vec::new();
    for (_, value) in trailers.iter().filter(|(k, _)| k.eq_ignore_ascii_case("co-authored-by")) {
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

impl Repository {
    /// Commits reachable from HEAD, newest first. Paged with `skip`/`limit`
    /// so the history list can load lazily as the user scrolls.
    pub fn log(&self, skip: usize, limit: usize) -> Result<Vec<Commit>> {
        if !self.has_head() {
            return Ok(Vec::new());
        }
        let repo = self.repo()?;
        let mut walk = repo.revwalk()?;
        walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        walk.push_head()?;
        walk.skip(skip).take(limit).map(|oid| -> Result<Commit> {
            let commit = repo.find_commit(oid?)?;
            let message = commit.message().unwrap_or_default();
            let author = commit.author();
            Ok(Commit {
                sha: commit.id().to_string(),
                short_sha: commit.as_object().short_id()?.as_str().unwrap_or_default().to_string(),
                author_name: author.name().unwrap_or_default().to_string(),
                author_email: author.email().unwrap_or_default().to_string(),
                time: author.when().seconds(),
                summary: commit.summary().ok().flatten().unwrap_or_default().to_string(),
                body: commit.body().ok().flatten().unwrap_or_default().trim_end().to_string(),
                parents: commit.parent_ids().map(|p| p.to_string()).collect(),
                co_authors: parse_co_authors(message),
            })
        }).collect()
    }

    /// Files changed by `sha` (against its first parent; root commits against
    /// the empty tree).
    pub fn commit_files(&self, sha: &str) -> Result<Vec<FileChange>> {
        let repo = self.repo()?;
        let commit = repo.revparse_single(sha)?.peel_to_commit()?;
        let parent = commit.parent(0).ok().map(|p| p.tree()).transpose()?;
        tree_changes(&repo, parent.as_ref(), &commit.tree()?)
    }
}

#[cfg(test)]
#[path = "tests/log.test.rs"]
mod tests;
