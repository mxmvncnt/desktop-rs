use gpui::{Entity, FocusHandle, SharedString, Subscription, Task, UniformListScrollHandle};

use super::overlay::Overlay;
use crate::features::accounts::AccountsStore;
use crate::features::branches::BranchUi;
use crate::features::changes::CommitBox;
use crate::features::diff::{DiffSelectUi, DiffView};
use crate::features::history::HistoryUi;
use crate::features::merge::MergeUi;
use crate::features::pickers::PopoverStore;
use crate::features::settings::{Settings, SettingsPage};
use crate::features::stashes::StashUi;
use crate::repo::RepoModel;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Popover {
    Repositories,
    Branches,
    NewBranch,
    Clone,
}

pub struct Workspace {
    pub settings: Settings,
    /// Sidebar width changed by a drag and not yet saved.
    pub(crate) sidebar_width_dirty: bool,
    pub settings_page: SettingsPage,
    /// Git identity + provider accounts (features/accounts).
    pub accounts: Entity<AccountsStore>,
    pub repo: Option<Entity<RepoModel>>,
    /// Open menus, dialogs, popovers and the settings modal (`overlay.rs`).
    pub overlays: Vec<Overlay>,
    /// Bumped on every overlay open so entrance animations replay.
    pub overlay_gen: usize,
    /// Toolbar popovers: cursor, filter / name / url inputs (features/pickers).
    pub pickers: Entity<PopoverStore>,
    /// History tab: dialog inputs, amend mode (features/history).
    pub history_ui: HistoryUi,
    /// Input of the stash dialogs (create / rename).
    pub stash_ui: StashUi,
    /// Merge dialog inputs and preview loading (features/merge).
    pub merge_ui: MergeUi,
    /// Input of the rename / create-from branch dialogs (features/branches).
    pub branch_ui: BranchUi,
    /// Window corner radius for this frame (0 unless floating under CSD);
    /// layers touching a window corner round their own background with it.
    pub corner_radius: f32,
    /// Error shown when no repository is open (e.g. bad path on startup).
    pub startup_error: Option<String>,

    /// Commit message input, amend mode and commit button (features/changes).
    pub commit_box: Entity<CommitBox>,

    /// List that ↑/↓ act on: the one last clicked (None = the tab's main list).
    pub nav_list: Option<super::navigation::NavList>,
    pub changes_scroll: UniformListScrollHandle,
    pub history_scroll: UniformListScrollHandle,
    pub diff_scroll: UniformListScrollHandle,
    /// Identity of the diff shown last; scroll resets when it changes.
    pub diff_key: Option<SharedString>,
    /// Diff options popover + split-row cache (see `features/diff`).
    pub diff_view: DiffView,
    pub diff_select: DiffSelectUi,
    /// Height of the selected commit's message area (history header).
    pub commit_msg: crate::features::history::MessageResize,
    /// Text selection in the commit header (summary and description).
    pub commit_sel: crate::features::history::CommitSelection,

    pub(super) focus: FocusHandle,
    /// Auto-dismiss timer of the current success notice (features/notices).
    pub(crate) notice_task: Option<Task<()>>,
    pub(super) repo_sub: Option<Subscription>,
    pub(super) repo_event_sub: Option<Subscription>,
    pub(super) _subs: Vec<Subscription>,
}
