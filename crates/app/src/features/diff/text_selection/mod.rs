//! Text selection in the diff: the selection model, the mouse binding of cells, and the `Workspace` handlers.

mod handlers;
pub(crate) mod model;
mod mouse;

pub(crate) use model::{TextSel, TextSide};
pub(super) use mouse::{TextSelect, TextTarget};
