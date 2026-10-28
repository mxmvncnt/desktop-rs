//! Toolbar popovers: repository list, branch list, new branch, clone.
//! Filter input on top, virtualized list, footer actions. Up/down + Enter work.
//! `PopoverStore` holds the cursor and inputs; `handlers.rs` builds the
//! components from it.

mod branch_picker;
mod clone_form;
mod filter;
mod handlers;
mod new_branch_base;
mod new_branch_base_list;
mod new_branch_form;
mod picker_shell;
mod repo_picker;
mod popover_handlers;
mod popover_layer;
mod store;
#[cfg(feature = "snapshot")]
pub mod snapshot;

pub use popover_layer::PopoverLayer;
pub use store::PopoverStore;
