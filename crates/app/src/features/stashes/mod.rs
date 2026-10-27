//! Stashes feature: list panel, detail pane, menu and dialogs.
//! Git work is in `gd-git/src/stash.rs`; see docs/ARCHITECTURE.md.

mod handlers;
mod ops;
mod stash_detail;
mod stash_dialog;
mod stash_header;
mod stash_list;
mod stash_menu;
mod stash_panel;
mod stash_panel_header;
mod stash_row;
mod state;
#[cfg(feature = "snapshot")]
pub mod snapshot;

pub use stash_dialog::StashDialogView;
pub use stash_panel::StashPanel;
pub use state::{StashDialog, StashState, StashUi};
