//! The two row lists (unified, split) and what they share: virtualized list helper, text cell layout, split-row model.

mod rows;
mod split;
mod split_model;
mod split_rows;
mod text_flow;
mod unified;
mod unified_rows;

pub(super) use split::SplitList;
pub(super) use split_model::{NONE, SplitDiff};
pub(super) use text_flow::{MARKER_W, TextFlow, row_box};
pub(super) use unified::UnifiedList;
