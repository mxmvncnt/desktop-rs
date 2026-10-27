//! Branch operations feature: branch context menu and the rename / delete /
//! create-from dialogs. Git work is in `gd-git/src/merge.rs`; the ops run
//! through `RepoModel::run`. See docs/ARCHITECTURE.md.

mod branch_dialog;
mod branch_menu_rows;
mod handlers;
mod ops;
mod state;

pub use branch_dialog::BranchDialogView;
#[cfg(feature = "snapshot")]
pub use state::BranchAction;
pub use state::BranchUi;
