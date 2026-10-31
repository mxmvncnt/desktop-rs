use gd_git::{Remote, RepoInfo};
use gd_ui::Icon;
use gpui::{App, AppContext as _, Entity};
use gpui_elements::editable_text::{EditableTextState, StringStorage};

use crate::repo::{LoadSlot, RepoModel};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RepoSettingsPage {
    #[default]
    Info,
    Remotes,
    Identity,
    Account,
}

impl RepoSettingsPage {
    pub const ALL: [RepoSettingsPage; 4] = [Self::Info, Self::Remotes, Self::Identity, Self::Account];

    pub fn label(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Remotes => "Remotes",
            Self::Identity => "Identity",
            Self::Account => "Account",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Self::Info => Icon::Info,
            Self::Remotes => Icon::Cloud,
            Self::Identity => Icon::Pen,
            Self::Account => Icon::Globe,
        }
    }
}

/// The open form or confirmation of a page; it closes when its write succeeds.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Edit {
    AddRemote,
    /// Editing the remote of this name.
    Remote(String),
    RemoveRemote(String),
}

#[derive(Clone)]
pub struct RepoSettingsInputs {
    pub remote_name: Entity<EditableTextState>,
    pub remote_url: Entity<EditableTextState>,
    pub remote_push_url: Entity<EditableTextState>,
}

/// What the repository settings modal shows; the identity and the account pick live in `AccountsStore`.
pub struct RepoSettingsStore {
    pub page: RepoSettingsPage,
    pub model: Option<Entity<RepoModel>>,
    pub info: Option<RepoInfo>,
    pub remotes: Vec<Remote>,
    pub edit: Option<Edit>,
    /// The last read or write error of the open page.
    pub error: Option<String>,
    pub inputs: RepoSettingsInputs,
    pub(super) load: LoadSlot,
}

impl RepoSettingsStore {
    pub fn new(cx: &mut App) -> Self {
        let mk = |cx: &mut App| cx.new(|cx| EditableTextState::new(StringStorage::default(), cx));
        Self {
            page: Default::default(),
            model: None,
            info: None,
            remotes: Vec::new(),
            edit: None,
            error: None,
            inputs: RepoSettingsInputs { remote_name: mk(cx), remote_url: mk(cx), remote_push_url: mk(cx) },
            load: LoadSlot::default(),
        }
    }
}
