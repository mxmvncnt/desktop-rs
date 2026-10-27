//! Pull-request preview: base branch, commit summary, merge-base diff and a
//! "Create pull request" link. Git work is in `gd-git/src/pr_preview.rs`.

mod ops;
mod pr_base_menu;
mod pr_header;
mod pr_preview_pane;
mod state;
#[cfg(feature = "snapshot")]
pub mod snapshot;

pub use state::PrPreviewState;
