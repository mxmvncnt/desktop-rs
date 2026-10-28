//! Force push preview: what a force push would send, drop and change on the
//! remote (file list + diff from the remote tip to HEAD), with the confirmation.
//! Git work is in `gd-git/src/history_ops.rs` (`force_push_preview`).

mod force_push_header;
mod force_push_pane;
mod ops;
mod state;
#[cfg(feature = "snapshot")]
pub mod snapshot;

pub use state::ForcePushState;
