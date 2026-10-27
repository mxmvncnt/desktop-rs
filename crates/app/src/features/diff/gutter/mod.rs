//! Line-number gutter: number cells with line checkboxes, hunk handles, and the partial-staging model.

mod block_handle;
mod line_selection_store;
mod number_cell;
mod style;

pub use line_selection_store::LineSelection;
pub(super) use block_handle::{BlockRow, RowSelect};
pub(super) use number_cell::number_cell;
pub(super) use style::{HANDLE_W, handle_bg, number_cell_w};
