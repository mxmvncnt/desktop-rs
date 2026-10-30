//! History feature: commit list, commit detail, context menu, dialogs and
//! amend mode. Git work is in `gd-git/src/history_ops.rs`; see
//! docs/ARCHITECTURE.md.

mod amend;
mod commit_detail;
mod commit_header;
mod commit_list;
mod commit_menu_rows;
mod commit_row;
mod commit_text;
mod confirm;
mod handlers;
mod history_dialog;
mod menu_actions;
mod message_resize;
mod multi_select;
mod ops;
mod search;
mod search_box;
mod selection;
#[cfg(feature = "snapshot")]
pub mod snapshot;
mod state;

pub use commit_list::CommitList;
pub use commit_text::CommitSelection;
pub use history_dialog::HistoryDialogView;
pub use message_resize::MessageResize;
pub use search::matching_commits;
pub use state::HistoryDialog;
pub use state::{HistoryState, HistoryUi};
