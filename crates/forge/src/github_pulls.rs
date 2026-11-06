//! GitHub pull requests: REST v3 pulls, issue comments, reviews and review comments.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::client::{Client, PAGE_SIZE};
use crate::error::Result;
use crate::model::User;
use crate::pull_model::*;
use crate::pull_query::{PullQuery, SortKey};
use crate::repo_ref::RepoRef;

#[derive(Deserialize)]
pub(crate) struct UserJson {
    login: String,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct Branch {
    #[serde(rename = "ref")]
    name: String,
    sha: String,
    repo: Option<RepoJson>,
}

#[derive(Deserialize)]
struct RepoJson {
    full_name: String,
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
    merged_at: Option<DateTime<Utc>>,
    user: Option<UserJson>,
    head: Branch,
    base: Branch,
    #[serde(default)]
    labels: Vec<Label>,
    #[serde(default)]
    assignees: Vec<UserJson>,
    #[serde(default)]
    requested_reviewers: Vec<UserJson>,
    /// Detail only.
    comments: Option<u32>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    html_url: String,
}

#[derive(Deserialize)]
struct SearchResults {
    items: Vec<SearchItem>,
}

/// An issue search result: no branches nor reviewers.
#[derive(Deserialize)]
pub(crate) struct SearchItem {
    number: u64,
    title: String,
    body: Option<String>,
    state: String,
    #[serde(default)]
    draft: bool,
    user: Option<UserJson>,
    #[serde(default)]
    labels: Vec<Label>,
    #[serde(default)]
    assignees: Vec<UserJson>,
    comments: Option<u32>,
    pull_request: Option<SearchPull>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    html_url: String,
}

#[derive(Deserialize)]
struct SearchPull {
    merged_at: Option<DateTime<Utc>>,
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
}

#[derive(Deserialize)]
pub(crate) struct ReviewComment {
    id: u64,
    in_reply_to_id: Option<u64>,
    user: Option<UserJson>,
    body: String,
    path: String,
    line: Option<u32>,
    original_line: Option<u32>,
    start_line: Option<u32>,
    side: Option<String>,
    created_at: Option<DateTime<Utc>>,
}

pub(crate) fn user(u: UserJson) -> User {
    User {
        name: None,
        username: u.login,
        avatar_url: u.avatar_url,
    }
}

pub(crate) fn pulls(
    c: &Client,
    repo: &RepoRef,
    filter: &PullFilter,
    page: u32,
) -> Result<PullPage> {
    if let Some(q) = filter.query.as_ref().filter(|q| !q.only_state()) {
        return search(c, repo, q, page);
    }
    // No merged state here: a closed pull request with a merge time was merged; `keep` sorts them out.
    let state = match filter.state() {
        Some(PullState::Open) => "open",
        Some(PullState::Closed | PullState::Merged) => "closed",
        None => "all",
    };
    let list: Vec<Pull> = c.get(
        &format!("/repos/{}/pulls", repo.full_path()),
        &[
            ("state", state.into()),
            ("sort", "updated".into()),
            ("direction", "desc".into()),
            ("per_page", PAGE_SIZE.to_string()),
            ("page", page.to_string()),
        ],
    )?;
    let more = list.len() >= PAGE_SIZE as usize;
    Ok(PullPage {
        pulls: filter.keep(list.into_iter().map(pull_from).collect()),
        more,
    })
}

/// The issue search, which runs GitHub's query syntax (`@me` included); `advanced_search` enables `OR` and parentheses.
fn search(c: &Client, repo: &RepoRef, q: &PullQuery, page: u32) -> Result<PullPage> {
    let mut query = vec![
        ("q", format!("repo:{} is:pr {}", repo.full_path(), q.raw)),
        ("advanced_search", "true".into()),
        ("per_page", PAGE_SIZE.to_string()),
        ("page", page.to_string()),
    ];
    let (key, desc) = q.sort.unwrap_or((SortKey::Updated, true));
    query.push((
        "sort",
        match key {
            SortKey::Created => "created",
            SortKey::Updated => "updated",
            SortKey::Comments => "comments",
        }
        .into(),
    ));
    query.push(("order", if desc { "desc" } else { "asc" }.into()));
    let found: SearchResults = c.get("/search/issues", &query)?;
    let more = found.items.len() >= PAGE_SIZE as usize;
    Ok(PullPage {
        pulls: found.items.into_iter().map(search_item).collect(),
        more,
    })
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
    let lines: Vec<ReviewComment> = c.get_all(&format!("/repos/{path}/pulls/{number}/comments"))?;
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

pub(crate) fn line_comment(
    c: &Client,
    repo: &RepoRef,
    pull: &PullRequest,
    target: &LineTarget,
    body: &str,
) -> Result<()> {
    let mut payload = json!({
        "body": body,
        "commit_id": pull.head_sha,
        "path": target.path,
        "line": target.end.line(),
        "side": side_name(target.end.side()),
    });
    if target.start != target.end {
        payload["start_line"] = json!(target.start.line());
        payload["start_side"] = json!(side_name(target.start.side()));
    }
    c.send(
        "POST",
        &format!("/repos/{}/pulls/{}/comments", repo.full_path(), pull.number),
        payload,
    )
}

pub(crate) fn reply(
    c: &Client,
    repo: &RepoRef,
    number: u64,
    thread: &Thread,
    body: &str,
) -> Result<()> {
    c.send(
        "POST",
        &format!(
            "/repos/{}/pulls/{number}/comments/{}/replies",
            repo.full_path(),
            thread.id
        ),
        json!({ "body": body }),
    )
}

fn side_name(side: Side) -> &'static str {
    match side {
        Side::Left => "LEFT",
        Side::Right => "RIGHT",
    }
}

pub(crate) fn pull_from(p: Pull) -> PullRequest {
    let state = match (p.state.as_str(), p.merged_at) {
        (_, Some(_)) => PullState::Merged,
        ("open", _) => PullState::Open,
        _ => PullState::Closed,
    };
    let from_fork = match (&p.head.repo, &p.base.repo) {
        (Some(head), Some(base)) => head.full_name != base.full_name,
        // The fork was deleted.
        (None, _) => true,
        _ => false,
    };
    PullRequest {
        number: p.number,
        title: p.title,
        body: p.body.unwrap_or_default(),
        state,
        draft: p.draft,
        author: p.user.map(user),
        head_ref: Some(p.head.name),
        head_sha: Some(p.head.sha),
        base_ref: Some(p.base.name),
        from_fork,
        labels: p.labels.into_iter().map(|l| l.name).collect(),
        assignees: p.assignees.into_iter().map(|u| u.login).collect(),
        reviewers: p.requested_reviewers.into_iter().map(|u| u.login).collect(),
        comments: p.comments,
        created_at: p.created_at,
        updated_at: p.updated_at,
        web_url: p.html_url,
        diff_refs: None,
    }
}

pub(crate) fn search_item(i: SearchItem) -> PullRequest {
    let merged = i.pull_request.is_some_and(|p| p.merged_at.is_some());
    PullRequest {
        number: i.number,
        title: i.title,
        body: i.body.unwrap_or_default(),
        state: match (i.state.as_str(), merged) {
            (_, true) => PullState::Merged,
            ("open", _) => PullState::Open,
            _ => PullState::Closed,
        },
        draft: i.draft,
        author: i.user.map(user),
        head_ref: None,
        head_sha: None,
        base_ref: None,
        from_fork: false,
        labels: i.labels.into_iter().map(|l| l.name).collect(),
        assignees: i.assignees.into_iter().map(|u| u.login).collect(),
        reviewers: Vec::new(),
        comments: i.comments,
        created_at: i.created_at,
        updated_at: i.updated_at,
        web_url: i.html_url,
        diff_refs: None,
    }
}

/// A review with something to say: its summary, or its verdict alone.
fn review_comment(r: Review) -> Option<Comment> {
    let verdict = match r.state.as_str() {
        "APPROVED" => Some("approved"),
        "CHANGES_REQUESTED" => Some("requested changes"),
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

/// Review comments grouped into threads: a reply points at the first comment of its thread.
pub(crate) fn threads(comments: Vec<ReviewComment>) -> Vec<Thread> {
    let mut threads: Vec<Thread> = Vec::new();
    for rc in comments {
        let comment = Comment {
            id: rc.id,
            author: rc.user.map(user),
            body: rc.body,
            created_at: rc.created_at,
            verdict: None,
        };
        let root = rc.in_reply_to_id.map(|id| id.to_string());
        match root.and_then(|id| threads.iter_mut().find(|t| t.id == id)) {
            Some(thread) => thread.comments.push(comment),
            None => threads.push(Thread {
                id: rc.id.to_string(),
                path: rc.path,
                side: if rc.side.as_deref() == Some("LEFT") {
                    Side::Left
                } else {
                    Side::Right
                },
                line: rc.line,
                start_line: rc.start_line.filter(|s| Some(*s) != rc.line),
                original_line: rc.original_line,
                resolved: false,
                comments: vec![comment],
            }),
        }
    }
    threads
}

#[cfg(test)]
#[path = "tests/github_pulls.test.rs"]
mod tests;
