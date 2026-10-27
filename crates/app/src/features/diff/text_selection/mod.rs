//! Text selection in the diff: the selection model, the mouse binding of cells, and the `Workspace` handlers.

mod handlers;
mod model;
mod mouse;

pub(super) use model::{TextPos, TextSel, TextSide};
pub(super) use mouse::{TextSelect, TextTarget};
