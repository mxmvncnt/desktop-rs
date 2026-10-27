//! Diff viewer: unified and split lists, line-selection gutter, hunk
//! expansion, syntax highlight, options popover. Git side is in
//! `gd-git/src/{engine,partial,expand}.rs`; see docs/ARCHITECTURE.md.
//!
//! - `view.rs`: `Workspace::render_diff`, the entry point of every pane; `state.rs`: `DiffView`, `DiffPane`;
//!   `handlers.rs`: `impl Workspace` setters and scrolling; `highlight.rs`: syntax runs.
//! - `list/` unified and split lists, `gutter/` numbers and line selection, `text_selection/`,
//!   `expand/` hunk expansion, `options/` display options.

mod expand;
mod gutter;
mod handlers;
mod highlight;
mod list;
mod options;
#[cfg(feature = "snapshot")]
pub mod snapshot;
mod state;
mod text_selection;
mod view;

pub use gutter::LineSelection;
pub use highlight::{DiffHighlight, HighlightCache, highlight_diff};
pub use options::{DiffOptionsButton, DiffOptionsMenu};
pub use state::{DiffPane, DiffSelectUi, DiffView};
