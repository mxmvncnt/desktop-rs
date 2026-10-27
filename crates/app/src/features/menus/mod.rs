//! Context menus of the Changes list (file rows, "N changed files" header) and
//! the discard confirm dialog. Rendered by `components::MenuView` / `gd_ui::Dialog`;
//! Esc/Enter/arrow routing is `workspace/overlay.rs`.

mod discard_dialog;
mod handlers;
mod menu_actions;
mod menu_rows;
mod state;

pub use discard_dialog::DiscardDialogView;
pub use state::ConfirmDialog;
