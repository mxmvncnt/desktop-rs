//! Merge state: the model side ([`MergeState`], a field of `RepoModel`) and the
//! Workspace side ([`MergeUi`]: inputs and preview loading).

use gd_git::{InProgress, MergePreview};
use gpui::{AppContext, Context, Entity, Subscription, UniformListScrollHandle};
use gpui_elements::editable_text::{EditableTextState, StringStorage, TextChanged};

use crate::repo::LoadSlot;
use crate::workspace::Workspace;

/// Which flavor of "merge into current branch" to run. The discriminant is the
/// index of the dialog tab.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MergeMode {
    Merge,
    Squash,
    Rebase,
}

impl MergeMode {
    pub const ALL: [MergeMode; 3] = [MergeMode::Merge, MergeMode::Squash, MergeMode::Rebase];
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Resolve {
    Ours,
    Theirs,
    /// Keep the working-tree contents (after editing by hand).
    Marked,
}

/// State added to `RepoModel` by the merge/conflict feature.
#[derive(Default)]
pub struct MergeState {
    /// Paused operation detected on the last refresh.
    pub in_progress: Option<InProgress>,
    /// A merge/continue/abort is running (disables the banner buttons).
    pub busy: Option<&'static str>,
}

/// The "Merge into current branch" dialog.
#[derive(Clone)]
pub struct MergeDialog {
    pub mode: MergeMode,
    /// Highlighted row in the filtered candidate list.
    pub ix: usize,
    pub preview: Option<MergePreview>,
    pub loading: bool,
}

impl MergeDialog {
    pub fn new(mode: MergeMode) -> Self {
        Self { mode, ix: 0, preview: None, loading: false }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConflictAction {
    Ours,
    Theirs,
    Resolved,
    OpenEditor,
    CopyPath,
    Reveal,
}

/// Everything the merge feature adds to `Workspace` besides its overlays
/// (the dialog and the conflict menu are `Overlay`s).
pub struct MergeUi {
    pub filter: Entity<EditableTextState>,
    /// Commit summary of the squash commit (Squash tab only).
    pub summary: Entity<EditableTextState>,
    /// Branch the summary default was last generated for.
    pub(super) summary_for: Option<String>,
    pub(super) scroll: UniformListScrollHandle,
    pub(super) preview_load: LoadSlot,
    _subs: Vec<Subscription>,
}

impl MergeUi {
    pub fn new(cx: &mut Context<Workspace>) -> Self {
        let filter = cx.new(|cx| EditableTextState::new(StringStorage::default(), cx));
        let summary = cx.new(|cx| EditableTextState::new(StringStorage::default(), cx));
        let subs = vec![cx.subscribe(&filter, |this: &mut Workspace, _, _: &TextChanged, cx| {
            if this.merge_dialog().is_some() {
                this.merge_select(0, cx);
            }
        })];
        Self {
            filter,
            summary,
            summary_for: None,
            scroll: UniformListScrollHandle::new(),
            preview_load: LoadSlot::default(),
            _subs: subs,
        }
    }
}
