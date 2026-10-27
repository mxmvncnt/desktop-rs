//! Context menu of the Changes list (file rows, "N changed files" header,
//! stash rows), the discard confirm dialog, and the Esc/Enter/arrow routing
//! across every open menu and dialog (`key_routing.rs`).
//! Built on `gd_ui::{ContextMenu, Dialog}`; only rendered while open.

mod discard_dialog;
mod handlers;
mod key_routing;
mod menu_actions;
mod menu_rows;
mod state;

pub use discard_dialog::DiscardDialogView;
pub use state::ConfirmDialog;
