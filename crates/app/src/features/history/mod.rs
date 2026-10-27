//! History feature: commit list, commit detail, context menu, dialogs and
//! amend mode. Git work is in `gd-git/src/history_ops.rs`; see
//! docs/ARCHITECTURE.md.

mod amend;
mod commit_detail;
mod commit_header;
mod commit_list;
mod commit_menu_rows;
mod commit_row;
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
mod state;

pub use commit_list::CommitList;
pub use history_dialog::HistoryDialogView;
pub use message_resize::MessageResize;
pub use search::matching_commits;
pub use state::HistoryDialog;
#[cfg(feature = "snapshot")]
pub use state::{ClickKind, HistoryAction};
#[cfg(feature = "snapshot")]
pub(crate) use handlers::menu_ctx;
pub use state::{HistoryState, HistoryUi};
