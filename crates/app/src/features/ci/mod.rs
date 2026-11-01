//! CI pipelines page: a full-window view (under the toolbar) modeled on GitLab's pipelines UI.
//! API work is in `gd-forge`; the account comes from `features/accounts`. See docs/ARCHITECTURE.md §CI page.

mod ci_page;
mod duration;
mod filter_entries;
mod filter_field;
mod handlers;
mod ops;
mod actions;
mod ansi;
mod breadcrumbs;
mod job_card;
mod job_sidebar;
mod job_view;
mod jobs_table;
mod log_selection;
mod log_view;
mod pipeline_row;
mod pipeline_view;
mod pipelines_header;
mod polling;
mod problem_view;
mod stage_column;
mod stage_dots;
mod state;
mod status_badge;
mod target;
#[cfg(feature = "snapshot")]
pub mod snapshot;

pub use ci_page::CiPage;
pub use state::{CiEvent, CiInputs, CiStore, Field};
