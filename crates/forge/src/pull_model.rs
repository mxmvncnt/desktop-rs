//! The normalized pull request model, in GitHub's vocabulary (GitLab calls them merge requests).

use chrono::{DateTime, Utc};

use crate::model::User;
use crate::pull_query::PullQuery;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PullState {
    Open,
    Closed,
    Merged,
}

/// The commits a GitLab line comment is positioned against (`diff_refs` of a merge request).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRefs {
    pub base_sha: String,
    pub start_sha: String,
    pub head_sha: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PullRequest {
    /// GitHub/Gitea `number`, GitLab `iid`.
    pub number: u64,
    pub title: String,
    pub body: String,
    pub state: PullState,
    pub draft: bool,
    pub author: Option<User>,
    /// Branch in the head repository. `None` in GitHub search results (`pull()` has it).
    pub head_ref: Option<String>,
    pub head_sha: Option<String>,
    pub base_ref: Option<String>,
    /// The head branch lives in another repository: it can't be checked out as a branch of this remote.
    pub from_fork: bool,
    pub labels: Vec<String>,
    /// Logins.
    pub assignees: Vec<String>,
    /// Logins of the requested reviewers (not on GitHub search results).
    pub reviewers: Vec<String>,
    /// Conversation comments, where the list reports them.
    pub comments: Option<u32>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub web_url: String,
    /// GitLab detail only.
    pub diff_refs: Option<DiffRefs>,
}

/// The side of a diff a review comment is on: the old file (deleted lines) or the new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Right,
}

/// The numbers of one diff line: a deleted line has only `old`, an added one only `new`, a context line both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LinePos {
    pub old: Option<u32>,
    pub new: Option<u32>,
}

impl LinePos {
    pub fn side(self) -> Side {
        if self.new.is_some() {
            Side::Right
        } else {
            Side::Left
        }
    }

    /// The number on `side()`.
    pub fn line(self) -> u32 {
        self.new.or(self.old).unwrap_or(0)
    }
}

/// Where a new review comment goes: one line (`start == end`) or a range of lines of one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineTarget {
    pub path: String,
    /// Path before a rename (`path` otherwise).
    pub old_path: String,
    pub start: LinePos,
    pub end: LinePos,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    pub id: u64,
    pub author: Option<User>,
    pub body: String,
    pub created_at: Option<DateTime<Utc>>,
    /// The verdict of a review whose summary this is ("approved", "changes requested").
    pub verdict: Option<String>,
}

/// Review comments on the same lines: the first one and its replies.
#[derive(Debug, Clone, PartialEq)]
pub struct Thread {
    /// GitHub: the first comment's id (replies go to it); GitLab: the discussion id; Gitea: the first comment's id.
    pub id: String,
    pub path: String,
    pub side: Side,
    /// Line on `side` in the current diff; `None` once the lines changed (outdated).
    pub line: Option<u32>,
    /// First line of a range.
    pub start_line: Option<u32>,
    /// The line when the comment was written, for outdated threads.
    pub original_line: Option<u32>,
    pub resolved: bool,
    pub comments: Vec<Comment>,
}

/// Everything said on a pull request: the comments of the conversation and the threads on lines.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Conversation {
    /// Oldest first.
    pub comments: Vec<Comment>,
    pub threads: Vec<Thread>,
}

#[derive(Debug, Clone, Default)]
pub struct PullFilter {
    /// A search; `None` = the open ones.
    pub query: Option<PullQuery>,
    /// The login `@me` stands for.
    pub me: String,
}

impl PullFilter {
    pub(crate) fn state(&self) -> Option<PullState> {
        match &self.query {
            Some(q) => q.state(),
            None => Some(PullState::Open),
        }
    }

    /// Keep what the query matches (the server filtered on the state at most).
    pub(crate) fn keep(&self, pulls: Vec<PullRequest>) -> Vec<PullRequest> {
        pulls
            .into_iter()
            .filter(|p| match &self.query {
                Some(q) => q.matches(p, &self.me),
                None => p.state == PullState::Open,
            })
            .collect()
    }
}

/// One page of the list. Filtering may leave a page short or empty with more after it.
#[derive(Debug, Clone, Default)]
pub struct PullPage {
    pub pulls: Vec<PullRequest>,
    pub more: bool,
}

/// The git ref a forge keeps the head of pull request `number` under, on the base repository (forks included).
pub fn pull_ref(gitlab: bool, number: u64) -> String {
    if gitlab {
        format!("refs/merge-requests/{number}/head")
    } else {
        format!("refs/pull/{number}/head")
    }
}
