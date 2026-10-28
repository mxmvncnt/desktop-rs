//! Virtualized list with the standard inset: the margin lives inside the
//! scrolled content, so rows clip at the list edge, not short of it.

use std::ops::Range;

use gpui::{App, IntoElement, UniformList, Window, prelude::*, px, uniform_list};

/// Total vertical inset (top + bottom); add it to fixed list heights.
pub const LIST_INSET_Y: f32 = 8.0;

pub fn list<R: IntoElement>(
    id: impl Into<gpui::ElementId>,
    count: usize,
    render: impl Fn(Range<usize>, &mut Window, &mut App) -> Vec<R> + 'static,
) -> UniformList {
    uniform_list(id, count, render).px(px(6.0)).py(px(LIST_INSET_Y / 2.0))
}
