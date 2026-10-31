//! Repository settings: a form modal (`Overlay::RepoSettings`) to view and edit one repository,
//! opened from the right-click menu of the repository button, the "No local changes" pane and
//! the Repository menu. Pages: info, identity override, account, ... Git side: `gd-git`.

mod handlers;
mod ignore_page;
mod info_page;
mod ops;
mod remote_form;
mod remotes_page;
mod repo_settings_modal;
mod state;
#[cfg(feature = "snapshot")]
pub mod snapshot;

pub use repo_settings_modal::RepoSettingsModal;
pub use state::RepoSettingsStore;
