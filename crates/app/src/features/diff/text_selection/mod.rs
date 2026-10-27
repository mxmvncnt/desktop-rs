//! Text selection in the diff: the selection model, the mouse binding of cells, and the `Workspace` handlers.

mod handlers;
pub(super) mod model;
mod mouse;

pub(super) use model::{TextSel, TextSide};
pub(super) use mouse::{TextSelect, TextTarget};
