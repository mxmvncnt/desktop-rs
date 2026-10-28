//! One row of the split diff: `[old text | old no · handle · new no | new text]`,
//! like Desktop. Plain fns, not `RenderOnce`: built for every visible row every
//! frame (hot-row exception, see docs/CONVENTIONS.md).

use gd_git::DiffLineKind;
use gd_ui::{Theme};
use gpui::{AnyElement, Div, IntoElement, ParentElement, Pixels, Styled, div, prelude::*, px};

use super::split_model::{NONE, SplitDiff, SplitRow};
use super::text_flow::{TextFlow, marker_cell, row_box, text_cell};
use super::unified_rows::{LineLook, line_look};
use crate::features::diff::expand::{Expander, header_hunk, trailing_row};
use crate::features::diff::gutter::number_cell;
use crate::features::diff::gutter::{BlockRow, RowSelect};
use crate::features::diff::highlight::{DiffHighlight, line_text};
use crate::features::diff::text_selection::TextSide;
use crate::features::diff::text_selection::{TextSelect, TextTarget};
use crate::repo::RepoModel;

/// What every split row needs besides its index.
pub struct SplitRows<'a> {
    pub split: &'a SplitDiff,
    pub hl: Option<&'a DiffHighlight>,
    pub sel: Option<(&'a RowSelect, &'a RepoModel)>,
    pub exp: Option<&'a Expander>,
    pub num_w: Pixels,
    /// Width of the expand-button gutter of header/trailing rows.
    pub exp_w: Pixels,
    pub flow: TextFlow,
    pub tsel: &'a TextSelect,
    pub theme: &'a Theme,
}

impl SplitRows<'_> {
    pub fn row(&self, ix: usize) -> AnyElement {
        let diff = &self.split.diff;
        if ix == self.split.rows.len() {
            return trailing_row(self.exp, diff, self.exp_w, self.flow, self.theme);
        }
        let row = self.split.rows[ix];
        if row.left == row.right && diff.lines[row.left as usize].kind == DiffLineKind::Hunk {
            return self.header_row(ix, row);
        }
        let (left, left_target) = self.side(row.left, TextSide::Left);
        let (right, right_target) = self.side(row.right, TextSide::Right);
        let targets = left_target.into_iter().chain(right_target).collect();
        self.tsel
            .bind_drag(row_box(self.flow), targets)
            .child(left)
            .child(self.gutter(ix, row))
            .child(right)
            .into_any_element()
    }

    /// Hunk headers span both columns and carry the expand buttons. They put
    /// them where the middle gutter is narrowest to reach: at the left edge,
    /// one gutter column wide.
    fn header_row(&self, ix: usize, row: SplitRow) -> AnyElement {
        let (diff, theme) = (&self.split.diff, self.theme);
        let line = &diff.lines[row.left as usize];
        let h = header_hunk(diff, row.left as usize).unwrap_or(0);
        row_box(self.flow)
            .bg(theme.diff_hunk_bg)
            .italic()
            .text_color(theme.text_faint)
            .when(ix > 0, |d| d.border_t_1().border_color(theme.border))
            .child(Expander::gutter(self.exp, diff, h, self.exp_w, theme).border_r_1().border_color(theme.border))
            .child(div().pl(px(8.0)).child(line.text.clone()))
            .into_any_element()
    }

    /// Middle gutter. Only the number of the side that owns a change (deletes
    /// left, adds right) is clickable.
    fn gutter(&self, ix: usize, row: SplitRow) -> Div {
        let (diff, theme, sel) = (&self.split.diff, self.theme, self.sel);
        let some = |i: u32| (i != NONE).then_some(i as usize);
        let (left, right) = (some(row.left), some(row.right));
        let old_no = left.and_then(|i| diff.lines[i].old_line);
        let new_no = right.and_then(|i| diff.lines[i].new_line);
        div()
            .flex()
            .flex_none()
            .border_l_1()
            .border_r_1()
            .border_color(theme.border)
            .child(number_cell(sel, diff, left, &[old_no], ("old-no", ix), self.num_w, theme))
            .when_some(sel, |d, (s, r)| {
                // A paired row belongs to the block of its changed side(s).
                let block = [left, right].into_iter().flatten().find_map(|i| RepoModel::change_block(diff, i));
                let first = block.as_ref().is_some_and(|b| left == Some(b.start) || right == Some(b.start));
                d.child(s.handle_cell(block, if first { BlockRow::First } else { BlockRow::Rest }, ix, r, theme))
            })
            .child(number_cell(sel, diff, right, &[new_no], ("new-no", ix), self.num_w, theme))
    }

    /// One text side: change marker + line text, and its hit-test target.
    fn side(&self, ix: u32, side: TextSide) -> (Div, Option<TextTarget>) {
        let theme = self.theme;
        let cell = div().flex().flex_1().min_w_0().overflow_hidden();
        if ix == NONE {
            return (cell.bg(theme.ink(0.025)), None);
        }
        let line = &self.split.diff.lines[ix as usize];
        let LineLook { bg, fg, marker, marker_color } = line_look(line.kind, theme);
        let spans = self.hl.map(|h| h.row(ix as usize)).unwrap_or(&[]);
        let selected = self.tsel.cols(ix as usize, side, line.text.len()).map(|r| (r, self.tsel.color));
        let (text, layout) = line_text(line, spans, selected, fg, theme);
        let target = TextTarget { line: ix as usize, side, layout };
        let text = self.tsel.bind_text(text_cell(text, self.flow), target.clone());
        (cell.bg(bg).child(marker_cell(marker, marker_color)).child(text), Some(target))
    }
}
