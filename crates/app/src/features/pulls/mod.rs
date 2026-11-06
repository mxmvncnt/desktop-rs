//! Pull requests page (merge requests on GitLab): a full-window view under the toolbar, like the CI page. The list, then a
//! pull request's conversation, commits and files, with comments on the conversation, on lines and replies to threads.
//! API work is in `gd-forge` (`pulls`), git work in `gd-git/src/pulls.rs`. See docs/ARCHITECTURE.md §Pull requests page.

mod comment_box;
mod comment_card;
mod commits_list;
mod conversation_view;
mod handlers;
mod ops;
mod pull_row;
mod pull_summary;
mod pulls_header;
mod pulls_page;
mod review;
mod review_notes;
#[cfg(feature = "snapshot")]
pub mod snapshot;
mod state;
mod state_badge;

pub use state::{PullsEvent, PullsStore};
