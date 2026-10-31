//! CI pipelines page: a full-window view (under the toolbar) modeled on GitLab's pipelines UI.
//! API work is in `gd-forge`; the account comes from `features/accounts`. See docs/ARCHITECTURE.md §CI page.

mod ci_page;
mod duration;
mod handlers;
mod ops;
mod pipeline_row;
mod pipelines_header;
mod polling;
mod problem_view;
mod stage_dots;
mod state;
mod status_badge;
mod target;
#[cfg(feature = "snapshot")]
pub mod snapshot;

pub use ci_page::CiPage;
pub use state::CiStore;
