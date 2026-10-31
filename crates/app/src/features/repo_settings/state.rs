use gd_git::RepoInfo;
use gd_ui::Icon;

use crate::repo::LoadSlot;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RepoSettingsPage {
    #[default]
    Info,
    Identity,
    Account,
}

impl RepoSettingsPage {
    pub const ALL: [RepoSettingsPage; 3] = [Self::Info, Self::Identity, Self::Account];

    pub fn label(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Identity => "Identity",
            Self::Account => "Account",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Self::Info => Icon::Info,
            Self::Identity => Icon::Pen,
            Self::Account => Icon::Globe,
        }
    }
}

/// What the repository settings modal shows; the identity and the account pick live in `AccountsStore`.
#[derive(Default)]
pub struct RepoSettingsStore {
    pub page: RepoSettingsPage,
    pub info: Option<RepoInfo>,
    pub error: Option<String>,
    pub(super) info_load: LoadSlot,
}
