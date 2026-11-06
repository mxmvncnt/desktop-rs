//! Pull requests page state: what is shown, the list loaded so far, and the open pull request with its conversation and head.

use std::path::PathBuf;

use gd_forge::{Conversation, PullRequest, PullState};
use gd_git::{PullCheckout, PullHead};
use gpui::{App, AppContext as _, Entity, EventEmitter, UniformListScrollHandle};
use gpui_elements::editable_text::{EditableTextState, StringStorage};

use crate::features::accounts::{Problem, Target};
use crate::repo::LoadSlot;

/// Which screen of the page is shown.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PullsView {
    List,
    Pull(u64),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PullTab {
    Conversation,
    Commits,
    Files,
}

/// Lines picked for a new review comment: indices of the shown diff's lines, in `path`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Pick {
    pub path: String,
    pub start: usize,
    pub end: usize,
}

/// Raised to the `Workspace`, which owns the banner and the repository.
pub enum PullsEvent {
    Notice(String),
    Error(String),
    /// Show the files of `base..head` in the review pane.
    Review {
        base: String,
        head: String,
    },
    Checkout(PullCheckout),
}

/// The text boxes of the page.
pub struct PullsInputs {
    /// A comment on the conversation.
    pub comment: Entity<EditableTextState>,
    /// A comment on the picked lines.
    pub line: Entity<EditableTextState>,
    /// A reply to a thread.
    pub reply: Entity<EditableTextState>,
}

pub struct PullsStore {
    pub open: bool,
    /// Repository the state below belongs to.
    pub(super) root: Option<PathBuf>,
    /// `None` while the account is being resolved.
    pub target: Option<Result<Target, Problem>>,
    pub view: PullsView,
    /// Screens left with Back, most recent last.
    pub(super) forward: Vec<PullsView>,
    pub tab: PullTab,
    /// State tab of the list (`None` = all).
    pub state: Option<PullState>,
    pub pulls: Vec<PullRequest>,
    pub page: u32,
    pub more: bool,
    pub loading: bool,
    pub refreshing: bool,
    pub list_error: Option<Problem>,
    pub list_scroll: UniformListScrollHandle,
    /// The open pull request, as `pull()` returned it.
    pub detail: Option<PullRequest>,
    pub detail_error: Option<Problem>,
    pub conversation: Option<Conversation>,
    pub conversation_error: Option<Problem>,
    /// Its head fetched into the repository; `Err` when git could not fetch it (the conversation still shows).
    pub head: Option<Result<PullHead, String>>,
    /// Commit shown on the Commits tab (index into `PullHead::commits`).
    pub commit: Option<usize>,
    pub pick: Option<Pick>,
    /// Id of the thread whose reply box is open.
    pub replying: Option<String>,
    /// A comment is being sent: the buttons are disabled.
    pub posting: bool,
    pub inputs: PullsInputs,
    pub conversation_scroll: gpui::ScrollHandle,
    pub commits_scroll: UniformListScrollHandle,
    /// Bumped when the repository changes: results of older requests are dropped.
    pub(super) epoch: u64,
    pub(super) resolve_load: LoadSlot,
    pub(super) list_load: LoadSlot,
    pub(super) detail_load: LoadSlot,
    pub(super) conversation_load: LoadSlot,
}

impl EventEmitter<PullsEvent> for PullsStore {}

impl PullsStore {
    pub fn new(cx: &mut App) -> Self {
        let input =
            |cx: &mut App| cx.new(|cx| EditableTextState::new(StringStorage::default(), cx));
        Self {
            open: false,
            root: None,
            target: None,
            view: PullsView::List,
            forward: Vec::new(),
            tab: PullTab::Conversation,
            state: Some(PullState::Open),
            pulls: Vec::new(),
            page: 1,
            more: false,
            loading: false,
            refreshing: false,
            list_error: None,
            list_scroll: UniformListScrollHandle::new(),
            detail: None,
            detail_error: None,
            conversation: None,
            conversation_error: None,
            head: None,
            commit: None,
            pick: None,
            replying: None,
            posting: false,
            inputs: PullsInputs {
                comment: input(cx),
                line: input(cx),
                reply: input(cx),
            },
            conversation_scroll: gpui::ScrollHandle::new(),
            commits_scroll: UniformListScrollHandle::new(),
            epoch: 0,
            resolve_load: LoadSlot::default(),
            list_load: LoadSlot::default(),
            detail_load: LoadSlot::default(),
            conversation_load: LoadSlot::default(),
        }
    }

    /// The resolved account and repository, if any.
    pub fn ready(&self) -> Option<&Target> {
        self.target.as_ref()?.as_ref().ok()
    }

    /// The fetched head of the open pull request.
    pub fn pull_head(&self) -> Option<&PullHead> {
        self.head.as_ref()?.as_ref().ok()
    }

    /// The open pull request: its detail, else its list entry.
    pub fn shown(&self) -> Option<&PullRequest> {
        let PullsView::Pull(number) = self.view else {
            return None;
        };
        self.detail
            .as_ref()
            .filter(|p| p.number == number)
            .or_else(|| self.pulls.iter().find(|p| p.number == number))
    }
}
