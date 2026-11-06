//! What a review diff adds to its rows, filled by `features/pulls`: line numbers that pick the lines of a new comment,
//! a wash over the picked lines, and blocks under rows (comment threads, the new comment box). A diff with notes is drawn
//! by the variable-height list, so a row can grow by its block.

use std::ops::RangeInclusive;
use std::rc::Rc;

use gd_ui::Theme;
use gpui::{
    AnyElement, App, ColorExt, InteractiveElement, IntoElement, MouseButton, ParentElement, Styled,
    Window, div, prelude::*,
};

/// A line number was clicked: the line, and whether Shift was down.
pub type PickFn = Rc<dyn Fn(usize, bool, &mut Window, &mut App)>;
/// The block under a line.
pub type BelowFn = Rc<dyn Fn(usize, &App) -> Option<AnyElement>>;

pub struct RowNotes {
    /// Lines (indices of `Diff::lines`) picked for a new comment.
    pub picked: Option<RangeInclusive<usize>>,
    /// A line number was clicked: the line, and whether Shift was down (extend the pick).
    pub on_pick: PickFn,
    /// The block under line `ix`, if any.
    pub below: BelowFn,
    /// Changes whenever a block appears, goes or changes height: the list then measures its rows again.
    pub key: u64,
}

impl RowNotes {
    /// A line-number cell of line `ix` that picks it.
    pub(super) fn pick_cell(&self, ix: usize, cell: AnyElement) -> AnyElement {
        let on_pick = self.on_pick.clone();
        div()
            .id(("note-pick", ix))
            .flex_none()
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, move |e, w, cx| {
                cx.stop_propagation();
                on_pick(ix, e.modifiers.shift, w, cx)
            })
            .child(cell)
            .into_any_element()
    }

    /// The row of `lines`, washed when one is picked, with the blocks under them.
    pub(super) fn wrap(&self, row: AnyElement, lines: &[usize], cx: &App) -> AnyElement {
        let picked = lines
            .iter()
            .any(|l| self.picked.as_ref().is_some_and(|p| p.contains(l)));
        let below: Vec<AnyElement> = lines.iter().filter_map(|l| (self.below)(*l, cx)).collect();
        if !picked && below.is_empty() {
            return row;
        }
        let wash = Theme::of(cx).accent.opacity(0.14);
        div()
            .flex()
            .flex_col()
            .w_full()
            .child(
                div()
                    .relative()
                    .child(row)
                    .when(picked, |d| d.child(div().absolute().inset_0().bg(wash))),
            )
            .children(below)
            .into_any_element()
    }
}
