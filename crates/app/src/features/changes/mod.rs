//! Changes tab: sidebar (tabs, file list, commit box) and the main pane
//! (selected file's diff, or "No local changes" suggestions).
//! RepoModel ops are in `ops.rs`; the diff itself is `features/diff`.

mod amend_notice;
mod change_row;
mod changes_header;
mod changes_list;
mod changes_pane;
mod changes_panel;
mod commit_box;
mod file_header;
mod no_changes;
mod ops;
mod resize;
mod sidebar;
mod sidebar_tabs;
#[cfg(feature = "snapshot")]
pub mod snapshot;
mod state;
mod undo_strip;

pub use commit_box::CommitBox;
pub use resize::SidebarResize;
pub use sidebar::Sidebar;
