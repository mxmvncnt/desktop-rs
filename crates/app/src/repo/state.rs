use std::collections::{HashMap, HashSet};
use std::time::{Instant, SystemTime};

use gd_git::{Branch, Commit, DiffOptions, FileChange, Repository, WorkingStatus};
use gpui::{Context, Task};

use super::diff_detail::DiffDetail;
use super::run::LoadSlot;
use crate::features::diff::HighlightCache;
use crate::watcher::RepoWatcher;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Changes,
    History,
}

/// Network/long-running operation in flight (drives toolbar busy state).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RemoteOp {
    Fetch,
    Pull,
    Push,
}

/// How much a refresh reloads.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum RefreshScope {
    /// Worktree status only (files edited).
    Status,
    /// Also branches, stashes, push state and history (git metadata changed).
    Full,
}

/// The commit made in this session, offered for undo (Desktop's strip above
/// the commit box). Hidden after a branch switch or an undo.
#[derive(Clone, Debug)]
pub struct LastCommit {
    pub summary: String,
    pub description: String,
    pub branch: String,
    /// Unix seconds.
    pub time: i64,
}

/// Emitted for the workspace, which owns the commit text inputs.
pub enum RepoEvent {
    /// The last commit was undone: put the message back in the commit box.
    CommitUndone { summary: String, description: String },
    /// Checkout requested while the working tree has changes: the workspace
    /// asks the user whether to stash them or bring them along.
    SwitchBranchWithChanges(Branch),
    /// A merge-like operation paused; git prepared this commit message.
    PreparedMessage { summary: String, description: String },
}

impl gpui::EventEmitter<RepoEvent> for RepoModel {}

pub struct RepoModel {
    pub repo: Repository,
    pub tab: Tab,
    /// App-wide diff options (set by the Workspace); changing them reloads the visible diffs.
    pub diff_options: DiffOptions,

    // ---- working directory ----
    pub status: Option<WorkingStatus>,
    /// Paths excluded from the next commit (Desktop's unchecked boxes). We
    /// store the *excluded* set so new files default to included.
    pub excluded: HashSet<String>,
    /// Files with only some lines selected (see `line_selection.rs`).
    pub partial: HashMap<String, crate::features::diff::LineSelection>,
    pub selected_change: Option<usize>,
    /// Diff of `selected_change` (its `files` stay empty: the list is `status`).
    pub change_detail: DiffDetail,
    /// What `change_detail.diff` was loaded for, so a reload of the same unchanged
    /// file keeps the (possibly expanded) diff on screen.
    pub change_diff_key: Option<(String, DiffOptions)>,

    // ---- refresh coalescing / live updates ----
    /// A refresh is running on the background executor.
    pub(super) refreshing: bool,
    /// A trigger arrived while refreshing: run exactly one more afterwards.
    pub(super) pending_refresh: Option<RefreshScope>,
    /// Number of refreshes started (debug counter; tests assert idle = stable).
    pub refresh_count: u64,
    /// When the file watcher last triggered a refresh (focus refresh skips if recent).
    pub last_watch_refresh: Option<Instant>,
    /// mtime of `.git/FETCH_HEAD`.
    pub last_fetch: Option<SystemTime>,
    pub(super) last_background_fetch: Option<Instant>,
    pub(super) _watcher: Option<RepoWatcher>,
    pub(super) _watch_task: Option<Task<()>>,
    pub(super) _background_fetch_task: Option<Task<()>>,

    // ---- history ----
    pub commits: Vec<Commit>,
    pub history_exhausted: bool,
    pub(super) history_loading: bool,
    pub selected_commit: Option<usize>,
    /// Files and diff of the selected commit (`base` = its sha).
    pub commit_detail: DiffDetail,
    /// Shared by every pane's highlighter.
    pub(super) hl_cache: HighlightCache,
    /// Push status, tags, multi-select (see `features/history`).
    pub history: crate::features::history::HistoryState,

    // ---- branches / remote ----
    pub branches: Vec<Branch>,
    pub remote_op: Option<RemoteOp>,
    pub committing: bool,
    pub last_commit: Option<LastCommit>,
    /// Last error from any operation, shown as a dismissible banner.
    pub error: Option<String>,
    /// Transient success message ("Committed 3 files").
    pub notice: Option<String>,

    /// Stash list, selection, and panel state (see features/stashes).
    pub stashes: crate::features::stashes::StashState,
    /// Pull-request preview (see `features/pr_preview`).
    pub pr: crate::features::pr_preview::PrPreviewState,
    /// Merge / rebase / conflict state (see `features/merge/`).
    pub merge: crate::features::merge::MergeState,
}

impl RepoModel {
    pub fn new(repo: Repository, cx: &mut Context<Self>) -> Self {
        let mut this = Self::empty(repo);
        this.refresh(cx);
        this.start_watcher(cx);
        this.start_background_fetch(cx);
        this
    }

    fn empty(repo: Repository) -> Self {
        Self {
            repo,
            tab: Tab::Changes,
            diff_options: DiffOptions::default(),
            status: None,
            excluded: HashSet::new(),
            partial: HashMap::new(),
            selected_change: None,
            change_detail: DiffDetail::default(),
            change_diff_key: None,
            refreshing: false,
            pending_refresh: None,
            refresh_count: 0,
            last_watch_refresh: None,
            last_fetch: None,
            last_background_fetch: None,
            _watcher: None,
            _watch_task: None,
            _background_fetch_task: None,
            commits: Vec::new(),
            history_exhausted: false,
            history_loading: false,
            selected_commit: None,
            commit_detail: DiffDetail { load: LoadSlot::late(), ..Default::default() },
            hl_cache: HighlightCache::default(),
            history: Default::default(),
            branches: Vec::new(),
            remote_op: None,
            committing: false,
            last_commit: None,
            error: None,
            notice: None,
            stashes: Default::default(),
            pr: Default::default(),
            merge: Default::default(),
        }
    }

    pub fn branch_name(&self) -> String {
        match &self.status {
            Some(s) => s.branch.clone().unwrap_or_else(|| "(detached HEAD)".into()),
            None => "…".into(),
        }
    }

    pub fn current_branch(&self) -> Option<&str> {
        self.status.as_ref()?.branch.as_deref()
    }

    /// A merge-like operation is paused and the next commit finishes it.
    pub fn merging(&self) -> bool {
        self.merge.in_progress.as_ref().is_some_and(|p| p.commit_finishes())
    }

    pub fn selected_change_file(&self) -> Option<&FileChange> {
        self.selected_change.and_then(|ix| self.changes().get(ix))
    }

    /// The history list is stale after the branch moved.
    pub(crate) fn reset_history_selection(&mut self) {
        self.commits.clear();
        self.selected_commit = None;
    }

    /// The undoable commit, only while still on the branch it was made on.
    pub fn undoable_commit(&self) -> Option<&LastCommit> {
        self.last_commit
            .as_ref()
            .filter(|c| self.current_branch() == Some(c.branch.as_str()))
    }

    pub fn changes(&self) -> &[FileChange] {
        self.status.as_ref().map(|s| s.files.as_slice()).unwrap_or(&[])
    }
}
