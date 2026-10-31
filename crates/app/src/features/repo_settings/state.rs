use std::path::PathBuf;

use gd_git::{ConfigEntry, Remote, RepoInfo, Repository};
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
    Ignore,
    Config,
}

impl RepoSettingsPage {
    pub const ALL: [RepoSettingsPage; 6] = [Self::Info, Self::Remotes, Self::Identity, Self::Account, Self::Ignore, Self::Config];

    pub fn label(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Remotes => "Remotes",
            Self::Identity => "Identity",
            Self::Account => "Account",
            Self::Ignore => "Ignored files",
            Self::Config => "Git config",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Self::Info => Icon::Info,
            Self::Remotes => Icon::Cloud,
            Self::Identity => Icon::Pen,
            Self::Account => Icon::Globe,
            Self::Ignore => Icon::EyeClosed,
            Self::Config => Icon::Tuning,
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
    AddConfig,
    /// Editing the value of this entry.
    Config(ConfigEntry),
}

/// The two files of ignore patterns of a repository.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IgnoreFile {
    /// `.gitignore` at the root, shared through git.
    Gitignore,
    /// `.git/info/exclude`, only for this clone.
    Exclude,
}

impl IgnoreFile {
    pub fn path(self, repo: &Repository) -> PathBuf {
        match self {
            Self::Gitignore => repo.root().join(".gitignore"),
            Self::Exclude => repo.common_git_dir().join("info/exclude"),
        }
    }
}

#[derive(Clone)]
pub struct RepoSettingsInputs {
    pub gitignore: Entity<EditableTextState>,
    pub exclude: Entity<EditableTextState>,
    pub config_key: Entity<EditableTextState>,
    pub config_value: Entity<EditableTextState>,
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
    pub config: Vec<ConfigEntry>,
    pub edit: Option<Edit>,
    /// The last read or write error of the open page.
    pub error: Option<String>,
    /// What the last successful write did ("Saved").
    pub notice: Option<&'static str>,
    pub inputs: RepoSettingsInputs,
    pub(super) load: LoadSlot,
    pub(super) ignore_load: LoadSlot,
}

impl RepoSettingsStore {
    pub fn new(cx: &mut App) -> Self {
        let mk = |cx: &mut App| cx.new(|cx| EditableTextState::new(StringStorage::default(), cx));
        Self {
            page: Default::default(),
            model: None,
            info: None,
            remotes: Vec::new(),
            config: Vec::new(),
            edit: None,
            error: None,
            notice: None,
            inputs: RepoSettingsInputs { config_key: mk(cx), config_value: mk(cx), gitignore: mk(cx), exclude: mk(cx), remote_name: mk(cx), remote_url: mk(cx), remote_push_url: mk(cx) },
            load: LoadSlot::default(),
            ignore_load: LoadSlot::default(),
        }
    }
}
