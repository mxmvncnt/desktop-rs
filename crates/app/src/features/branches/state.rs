//! Branch-ops state: the right-click menu and the rename / delete /
//! create-from dialogs ([`BranchUi`], a plain `Workspace` field).

use gd_git::Branch;
use gpui::{AppContext, Context, Entity, Subscription};
use gpui_elements::editable_text::{EditableTextState, StringStorage, TextChanged};

use crate::workspace::Workspace;

#[derive(Clone)]
pub enum BranchDialog {
    Rename { old: String },
    /// `remote` is the upstream (`origin/x`) that can be deleted along with it.
    Delete { branch: Branch, remote: Option<String>, also_remote: bool },
    CreateFrom { base: String },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BranchAction {
    Merge,
    Rebase,
    CreateFrom,
    Rename,
    CopyName,
    Delete,
}

/// A branch on a remote, to delete on the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteBranch {
    pub remote: String,
    pub branch: String,
}

impl RemoteBranch {
    /// Parse `origin/feature/x` (remote name = first segment).
    pub fn parse(full: &str) -> Option<Self> {
        let (remote, branch) = full.split_once('/')?;
        Some(Self { remote: remote.to_string(), branch: branch.to_string() })
    }
}

/// What the branch ops add to `Workspace` besides their overlays (the menu
/// and the dialogs).
pub struct BranchUi {
    /// Name typed into the rename / create-from dialogs.
    pub input: Entity<EditableTextState>,
    _subs: Vec<Subscription>,
}

impl BranchUi {
    pub fn new(cx: &mut Context<Workspace>) -> Self {
        let input = cx.new(|cx| EditableTextState::new(StringStorage::default(), cx));
        let subs = vec![cx.subscribe(&input, |_: &mut Workspace, _, _: &TextChanged, cx| cx.notify())];
        Self { input, _subs: subs }
    }
}
