//! GitLab merge requests: the list, the detail (with `diff_refs`), discussions and notes.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::client::{Client, PAGE_SIZE};
use crate::error::Result;
use crate::model::User;
use crate::pull_model::*;
use crate::pull_query::SortKey;
use crate::repo_ref::RepoRef;

#[derive(Deserialize)]
struct UserJson {
    name: Option<String>,
    username: String,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct RefsJson {
    base_sha: Option<String>,
    start_sha: Option<String>,
    head_sha: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct Mr {
    iid: u64,
    title: String,
    description: Option<String>,
    state: String,
    #[serde(default, alias = "work_in_progress")]
    draft: bool,
    author: Option<UserJson>,
    source_branch: String,
    target_branch: String,
    sha: Option<String>,
    source_project_id: u64,
    target_project_id: u64,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    assignees: Vec<UserJson>,
    #[serde(default)]
    reviewers: Vec<UserJson>,
    user_notes_count: Option<u32>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    web_url: String,
    diff_refs: Option<RefsJson>,
    milestone: Option<MilestoneJson>,
    detailed_merge_status: Option<String>,
}

#[derive(Deserialize)]
struct MilestoneJson {
    title: String,
}

#[derive(Deserialize)]
struct Approvals {
    #[serde(default)]
    approved_by: Vec<Approver>,
}

#[derive(Deserialize)]
struct Approver {
    user: UserJson,
}

#[derive(Deserialize)]
pub(crate) struct Discussion {
    id: String,
    notes: Vec<Note>,
}

#[derive(Deserialize)]
struct Note {
    id: u64,
    #[serde(rename = "type")]
    kind: Option<String>,
    body: String,
    author: Option<UserJson>,
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    system: bool,
    #[serde(default)]
    resolved: bool,
    position: Option<Position>,
}

#[derive(Deserialize)]
struct Position {
    new_path: Option<String>,
    old_path: Option<String>,
    new_line: Option<u32>,
    old_line: Option<u32>,
    line_range: Option<LineRange>,
}

#[derive(Deserialize)]
struct LineRange {
    start: Option<RangeEnd>,
}

#[derive(Deserialize)]
struct RangeEnd {
    new_line: Option<u32>,
    old_line: Option<u32>,
}

fn user(u: UserJson) -> User {
    User {
        name: u.name,
        username: u.username,
        avatar_url: u.avatar_url,
    }
}

pub(crate) fn pulls(
    c: &Client,
    repo: &RepoRef,
    filter: &PullFilter,
    page: u32,
) -> Result<PullPage> {
    // GitLab's `closed` leaves the merged ones out; GitHub's (the query's) has them.
    let state = match filter.state() {
        Some(PullState::Open) => "opened",
        Some(PullState::Merged) => "merged",
        Some(PullState::Closed) | None => "all",
    };
    let q = filter.query.as_ref();
    let (order, desc) = match q.and_then(|q| q.sort) {
        Some((SortKey::Created, desc)) => ("created_at", desc),
        Some((_, desc)) => ("updated_at", desc),
        None => ("updated_at", true),
    };
    let mut query = vec![
        ("state", state.to_string()),
        ("order_by", order.into()),
        ("sort", if desc { "desc" } else { "asc" }.into()),
        ("per_page", PAGE_SIZE.to_string()),
        ("page", page.to_string()),
    ];
    if let Some(author) = q.and_then(|q| q.author(&filter.me)) {
        query.push(("author_username", author));
    }
    let list: Vec<Mr> = c.get(
        &format!("/projects/{}/merge_requests", repo.gitlab_id()),
        &query,
    )?;
    let more = list.len() >= PAGE_SIZE as usize;
    Ok(PullPage {
        pulls: filter.keep(list.into_iter().map(pull_from).collect()),
        more,
    })
}

pub(crate) fn pull(c: &Client, repo: &RepoRef, number: u64) -> Result<PullRequest> {
    Ok(pull_from(c.get(
        &format!("/projects/{}/merge_requests/{number}", repo.gitlab_id()),
        &[],
    )?))
}

pub(crate) fn conversation(c: &Client, repo: &RepoRef, number: u64) -> Result<Conversation> {
    let mr = format!("/projects/{}/merge_requests/{number}", repo.gitlab_id());
    let discussions: Vec<Discussion> = c.get_all(&format!("{mr}/discussions"))?;
    // Approvals are not in every edition's API; without them the reviewers have no verdict.
    let approvals = c.get::<Approvals>(&format!("{mr}/approvals"), &[]).ok();
    let mut out = conversation_from(discussions);
    out.reviews = latest_verdicts(
        approvals
            .into_iter()
            .flat_map(|a| a.approved_by)
            .map(|a| (user(a.user), Verdict::Approved)),
    );
    Ok(out)
}

pub(crate) fn comment(c: &Client, repo: &RepoRef, number: u64, body: &str) -> Result<()> {
    c.send(
        "POST",
        &format!(
            "/projects/{}/merge_requests/{number}/notes",
            repo.gitlab_id()
        ),
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
    let refs = pull.diff_refs.as_ref().ok_or(crate::Error::Unsupported(
        "commenting on lines without the merge request's diff refs",
    ))?;
    c.send(
        "POST",
        &format!(
            "/projects/{}/merge_requests/{}/discussions",
            repo.gitlab_id(),
            pull.number
        ),
        json!({ "body": body, "position": position(refs, target) }),
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
            "/projects/{}/merge_requests/{number}/discussions/{}/notes",
            repo.gitlab_id(),
            thread.id
        ),
        json!({ "body": body }),
    )
}

/// A text position: the end line's numbers (both for a context line), and for a range its `line_range`,
/// whose ends are named by GitLab's line code `sha1(path)_old_new`.
pub(crate) fn position(refs: &DiffRefs, target: &LineTarget) -> Value {
    let mut pos = json!({
        "position_type": "text",
        "base_sha": refs.base_sha,
        "start_sha": refs.start_sha,
        "head_sha": refs.head_sha,
        "old_path": target.old_path,
        "new_path": target.path,
    });
    if let Some(n) = target.end.old {
        pos["old_line"] = json!(n);
    }
    if let Some(n) = target.end.new {
        pos["new_line"] = json!(n);
    }
    if target.start != target.end {
        let file = sha1_smol::Sha1::from(&target.path).hexdigest();
        let end = |p: LinePos| {
            json!({
                "line_code": format!(
                    "{file}_{}_{}",
                    p.old.map(|n| n.to_string()).unwrap_or_default(),
                    p.new.map(|n| n.to_string()).unwrap_or_default()
                ),
                "type": match (p.old, p.new) {
                    (None, Some(_)) => Value::from("new"),
                    (Some(_), None) => Value::from("old"),
                    _ => Value::Null,
                },
                "old_line": p.old,
                "new_line": p.new,
            })
        };
        pos["line_range"] = json!({ "start": end(target.start), "end": end(target.end) });
    }
    pos
}

pub(crate) fn pull_from(m: Mr) -> PullRequest {
    let state = match m.state.as_str() {
        "opened" | "locked" => PullState::Open,
        "merged" => PullState::Merged,
        _ => PullState::Closed,
    };
    let diff_refs = m.diff_refs.and_then(|r| {
        Some(DiffRefs {
            base_sha: r.base_sha?,
            start_sha: r.start_sha?,
            head_sha: r.head_sha?,
        })
    });
    PullRequest {
        number: m.iid,
        title: m.title,
        body: m.description.unwrap_or_default(),
        state,
        draft: m.draft,
        author: m.author.map(user),
        head_ref: Some(m.source_branch),
        head_sha: m.sha,
        base_ref: Some(m.target_branch),
        from_fork: m.source_project_id != m.target_project_id,
        labels: m.labels,
        assignees: m.assignees.into_iter().map(|u| u.username).collect(),
        reviewers: m.reviewers.into_iter().map(|u| u.username).collect(),
        comments: m.user_notes_count,
        created_at: m.created_at,
        updated_at: m.updated_at,
        web_url: m.web_url,
        diff_refs,
        milestone: m.milestone.map(|m| m.title),
        merge: (state == PullState::Open).then(|| {
            match (m.draft, m.detailed_merge_status.as_deref()) {
                (true, _) | (_, Some("draft_status")) => MergeStatus::Draft,
                (_, Some("mergeable")) => MergeStatus::Mergeable,
                (_, Some("conflict" | "broken_status")) => MergeStatus::Conflicts,
                (_, Some("ci_must_pass" | "ci_still_running")) => MergeStatus::ChecksPending,
                (_, Some("need_rebase")) => MergeStatus::Behind,
                (_, Some("checking" | "unchecked" | "preparing" | "approvals_syncing") | None) => {
                    MergeStatus::Checking
                }
                // not_approved, discussions_not_resolved, blocked_status, jira, …
                _ => MergeStatus::Blocked,
            }
        }),
    }
}

/// Discussions on a position are threads; the notes of the others are the conversation. System notes ("added 1 commit") are left out.
pub(crate) fn conversation_from(discussions: Vec<Discussion>) -> Conversation {
    let mut out = Conversation::default();
    for d in discussions {
        let notes: Vec<Note> = d.notes.into_iter().filter(|n| !n.system).collect();
        let Some(first) = notes.first() else {
            continue;
        };
        let position = (first.kind.as_deref() == Some("DiffNote"))
            .then_some(first.position.as_ref())
            .flatten();
        let Some(pos) = position else {
            out.comments.extend(notes.into_iter().map(comment_from));
            continue;
        };
        let side = if pos.new_line.is_some() {
            Side::Right
        } else {
            Side::Left
        };
        let start = pos.line_range.as_ref().and_then(|r| r.start.as_ref());
        let line = pos.new_line.or(pos.old_line);
        out.threads.push(Thread {
            id: d.id,
            path: pos
                .new_path
                .clone()
                .or(pos.old_path.clone())
                .unwrap_or_default(),
            side,
            line,
            start_line: start
                .and_then(|s| match side {
                    Side::Right => s.new_line,
                    Side::Left => s.old_line,
                })
                .filter(|s| Some(*s) != line),
            original_line: line,
            resolved: first.resolved,
            comments: notes.into_iter().map(comment_from).collect(),
        });
    }
    out.comments.sort_by_key(|c| c.created_at);
    out
}

fn comment_from(n: Note) -> Comment {
    Comment {
        id: n.id,
        author: n.author.map(user),
        body: n.body,
        created_at: n.created_at,
        verdict: None,
    }
}

#[cfg(test)]
#[path = "tests/gitlab_pulls.test.rs"]
mod tests;
