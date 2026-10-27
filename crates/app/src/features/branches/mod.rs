//! Branch operations feature: branch context menu and the rename / delete /
//! create-from dialogs. Git work is in `gd-git/src/merge.rs`; the ops run
//! through `RepoModel::run`. See docs/ARCHITECTURE.md.

mod branch_dialog;
mod branch_menu_rows;
mod handlers;
mod ops;
#[cfg(feature = "snapshot")]
pub mod snapshot;
mod state;

pub use branch_dialog::BranchDialogView;
pub use state::{BranchDialog, BranchUi};
