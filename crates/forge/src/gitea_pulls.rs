//! Gitea and Forgejo pull requests: pulls, issue comments, reviews and their comments.
//! Review comments are not threaded in the API: comments on the same line form a thread, and a reply is a new comment there.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::client::{Client, PAGE_SIZE};
use crate::error::Result;
use crate::model::User;
use crate::pull_model::*;
use crate::repo_ref::RepoRef;

#[derive(Deserialize)]
struct UserJson {
    login: String,
    full_name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct Branch {
    #[serde(rename = "ref")]
    name: String,
    sha: Option<String>,
    repo_id: Option<u64>,
}

#[derive(Deserialize)]
struct Label {
    name: String,
}

#[derive(Deserialize)]
pub(crate) struct Pull {
    number: u64,
    title: String,
    body: Option<String>,
    state: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    merged: bool,
    user: Option<UserJson>,
    head: Branch,
    base: Branch,
    #[serde(default)]
    labels: Vec<Label>,
    comments: Option<u32>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    html_url: String,
}

#[derive(Deserialize)]
struct IssueComment {
    id: u64,
    user: Option<UserJson>,
    body: Option<String>,
    created_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct Review {
    id: u64,
    user: Option<UserJson>,
    body: Option<String>,
    state: String,
    submitted_at: Option<DateTime<Utc>>,
    #[serde(default)]
    comments_count: u32,
}

#[derive(Deserialize)]
pub(crate) struct ReviewComment {
    id: u64,
    user: Option<UserJson>,
    body: String,
    path: String,
    /// Line on the new side, 0 when the comment is on the old side.
    #[serde(default)]
    position: u32,
    /// Line on the old side.
    #[serde(default)]
    original_position: u32,
    resolver: Option<UserJson>,
    created_at: Option<DateTime<Utc>>,
}

fn user(u: UserJson) -> User {
    User {
        name: u.full_name.filter(|n| !n.is_empty()),
        username: u.login,
        avatar_url: u.avatar_url,
    }
}

pub(crate) fn pulls(
    c: &Client,
    repo: &RepoRef,
    filter: &PullFilter,
    page: u32,
) -> Result<Vec<PullRequest>> {
    let state = match filter.state {
        Some(PullState::Open) => "open",
        Some(PullState::Closed | PullState::Merged) => "closed",
        None => "all",
    };
    let list: Vec<Pull> = c.get(
        &format!("/repos/{}/pulls", repo.full_path()),
        &[
            ("state", state.into()),
            ("sort", "recentupdate".into()),
            ("limit", PAGE_SIZE.to_string()),
            ("page", page.to_string()),
        ],
    )?;
    let wanted = filter.state;
    Ok(list
        .into_iter()
        .map(pull_from)
        .filter(|p| match wanted {
            Some(PullState::Closed) => p.state == PullState::Closed,
            Some(PullState::Merged) => p.state == PullState::Merged,
            _ => true,
        })
        .collect())
}

pub(crate) fn pull(c: &Client, repo: &RepoRef, number: u64) -> Result<PullRequest> {
    Ok(pull_from(c.get(
        &format!("/repos/{}/pulls/{number}", repo.full_path()),
        &[],
    )?))
}

pub(crate) fn conversation(c: &Client, repo: &RepoRef, number: u64) -> Result<Conversation> {
    let path = repo.full_path();
    let issue: Vec<IssueComment> = c.get_all(&format!("/repos/{path}/issues/{number}/comments"))?;
    let reviews: Vec<Review> = c.get_all(&format!("/repos/{path}/pulls/{number}/reviews"))?;
    let mut lines: Vec<ReviewComment> = Vec::new();
    for r in reviews.iter().filter(|r| r.comments_count > 0) {
        lines.extend(c.get::<Vec<ReviewComment>>(
            &format!("/repos/{path}/pulls/{number}/reviews/{}/comments", r.id),
            &[],
        )?);
    }
    let mut comments: Vec<Comment> = issue
        .into_iter()
        .map(|i| Comment {
            id: i.id,
            author: i.user.map(user),
            body: i.body.unwrap_or_default(),
            created_at: i.created_at,
            verdict: None,
        })
        .chain(reviews.into_iter().filter_map(review_comment))
        .collect();
    comments.sort_by_key(|c| c.created_at);
    Ok(Conversation {
        comments,
        threads: threads(lines),
    })
}

pub(crate) fn comment(c: &Client, repo: &RepoRef, number: u64, body: &str) -> Result<()> {
    c.send(
        "POST",
        &format!("/repos/{}/issues/{number}/comments", repo.full_path()),
        json!({ "body": body }),
    )
}

/// A review of one comment. No ranges: the comment goes on the last line.
pub(crate) fn line_comment(
    c: &Client,
    repo: &RepoRef,
    pull: &PullRequest,
    target: &LineTarget,
    body: &str,
) -> Result<()> {
    let (old, new) = match target.end.side() {
        Side::Left => (target.end.line(), 0),
        Side::Right => (0, target.end.line()),
    };
    c.send(
        "POST",
        &format!("/repos/{}/pulls/{}/reviews", repo.full_path(), pull.number),
        json!({
            "event": "COMMENT",
            "body": "",
            "commit_id": pull.head_sha,
            "comments": [{
                "path": target.path,
                "body": body,
                "old_position": old,
                "new_position": new,
            }],
        }),
    )
}

/// A new comment on the thread's line.
pub(crate) fn reply(
    c: &Client,
    repo: &RepoRef,
    pull: &PullRequest,
    thread: &Thread,
    body: &str,
) -> Result<()> {
    let line = thread.line.ok_or(crate::Error::Unsupported(
        "replying to an outdated thread",
    ))?;
    let pos = match thread.side {
        Side::Left => LinePos {
            old: Some(line),
            new: None,
        },
        Side::Right => LinePos {
            old: None,
            new: Some(line),
        },
    };
    let target = LineTarget {
        path: thread.path.clone(),
        old_path: thread.path.clone(),
        start: pos,
        end: pos,
    };
    line_comment(c, repo, pull, &target, body)
}

pub(crate) fn pull_from(p: Pull) -> PullRequest {
    let state = match (p.state.as_str(), p.merged) {
        (_, true) => PullState::Merged,
        ("open", _) => PullState::Open,
        _ => PullState::Closed,
    };
    PullRequest {
        number: p.number,
        title: p.title,
        body: p.body.unwrap_or_default(),
        state,
        draft: p.draft,
        author: p.user.map(user),
        head_ref: Some(p.head.name),
        head_sha: p.head.sha,
        base_ref: Some(p.base.name),
        from_fork: p.head.repo_id != p.base.repo_id,
        labels: p.labels.into_iter().map(|l| l.name).collect(),
        comments: p.comments,
        created_at: p.created_at,
        updated_at: p.updated_at,
        web_url: p.html_url,
        diff_refs: None,
    }
}

fn review_comment(r: Review) -> Option<Comment> {
    let verdict = match r.state.as_str() {
        "APPROVED" => Some("approved"),
        "REQUEST_CHANGES" => Some("requested changes"),
        _ => None,
    };
    let body = r.body.unwrap_or_default();
    (verdict.is_some() || !body.trim().is_empty()).then(|| Comment {
        id: r.id,
        author: r.user.map(user),
        body,
        created_at: r.submitted_at,
        verdict: verdict.map(str::to_string),
    })
}

/// Comments on the same path, side and line, oldest first, are one thread.
pub(crate) fn threads(mut comments: Vec<ReviewComment>) -> Vec<Thread> {
    comments.sort_by_key(|c| (c.created_at, c.id));
    let mut threads: Vec<Thread> = Vec::new();
    for rc in comments {
        let (side, line) = if rc.position > 0 {
            (Side::Right, rc.position)
        } else {
            (Side::Left, rc.original_position)
        };
        let resolved = rc.resolver.is_some();
        let comment = Comment {
            id: rc.id,
            author: rc.user.map(user),
            body: rc.body,
            created_at: rc.created_at,
            verdict: None,
        };
        match threads
            .iter_mut()
            .find(|t| t.path == rc.path && t.side == side && t.line == Some(line))
        {
            Some(t) => {
                t.resolved |= resolved;
                t.comments.push(comment);
            }
            None => threads.push(Thread {
                id: rc.id.to_string(),
                path: rc.path,
                side,
                line: (line > 0).then_some(line),
                start_line: None,
                original_line: (line > 0).then_some(line),
                resolved,
                comments: vec![comment],
            }),
        }
    }
    threads
}

#[cfg(test)]
#[path = "tests/gitea_pulls.test.rs"]
mod tests;
