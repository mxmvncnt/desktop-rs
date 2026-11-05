//! One row of the split diff: `[old text | old no · handle · new no | new text]`,
//! like Desktop. Plain fns, not `RenderOnce`: built for every visible row every
//! frame (hot-row exception, see docs/CONVENTIONS.md).

use gd_git::DiffLineKind;
use gd_ui::Theme;
use gpui::{AnyElement, Div, IntoElement, ParentElement, Pixels, Styled, div, prelude::*, px};

use super::split_model::{NONE, SplitDiff, SplitRow};
use super::text_flow::{MARKER_W, TextFlow, marker_cell, row_box, text_cell};
use super::unified_rows::{LineLook, line_look};
use crate::features::diff::expand::{Expander, header_hunk, trailing_row};
use crate::features::diff::gutter::NumberCell;
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
        // Sides and gutter go straight into the row, without a box each.
        let mut targets = Vec::new();
        let el = self.side(row_box(self.flow), row.left, TextSide::Left, &mut targets);
        let el = self.gutter(el, ix, row);
        let el = self.side(el, row.right, TextSide::Right, &mut targets);
        self.tsel.bind_drag(el, targets).into_any_element()
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
            .child(
                Expander::gutter(self.exp, diff, h, self.exp_w, theme)
                    .border_r_1()
                    .border_color(theme.border),
            )
            .child(div().pl(px(8.0)).child(line.text.clone()))
            .into_any_element()
    }

    /// Middle gutter. Only the number of the side that owns a change (deletes
    /// left, adds right) is clickable. Its two number cells paint its borders.
    fn gutter(&self, el: Div, ix: usize, row: SplitRow) -> Div {
        let (diff, theme, sel) = (&self.split.diff, self.theme, self.sel);
        let some = |i: u32| (i != NONE).then_some(i as usize);
        let (left, right) = (some(row.left), some(row.right));
        let old_no = left.and_then(|i| diff.lines[i].old_line);
        let new_no = right.and_then(|i| diff.lines[i].new_line);
        let cell = |row, numbers, id, edges| {
            NumberCell {
                select: sel,
                diff,
                row,
                numbers,
                id,
                num_w: self.num_w,
                edges,
                theme,
            }
            .build()
        };
        el.child(cell(left, &[old_no], ("old-no", ix), [true, false]))
            .when_some(sel, |d, (s, r)| {
                // A paired row belongs to the block of its changed side(s).
                let block = [left, right]
                    .into_iter()
                    .flatten()
                    .find_map(|i| RepoModel::change_block(diff, i));
                let first = block
                    .as_ref()
                    .is_some_and(|b| left == Some(b.start) || right == Some(b.start));
                d.child(s.handle_cell(
                    block,
                    if first {
                        BlockRow::First
                    } else {
                        BlockRow::Rest
                    },
                    ix,
                    r,
                    theme,
                ))
            })
            .child(cell(right, &[new_no], ("new-no", ix), [false, true]))
    }

    /// One text side: change marker + line text, and its hit-test target. The
    /// marker and the text cell each paint their part of the line background.
    /// An empty side is one box as wide as a marker plus a text cell.
    fn side(&self, el: Div, ix: u32, side: TextSide, targets: &mut Vec<TextTarget>) -> Div {
        let theme = self.theme;
        if ix == NONE {
            return el.child(
                div()
                    .flex_1()
                    .flex_basis(px(MARKER_W))
                    .min_w_0()
                    .bg(theme.diff_empty_bg),
            );
        }
        let line = &self.split.diff.lines[ix as usize];
        let LineLook {
            bg,
            fg,
            marker,
            marker_color,
        } = line_look(line.kind, theme);
        let spans = self.hl.map(|h| h.row(ix as usize)).unwrap_or(&[]);
        let selected = self
            .tsel
            .cols(ix as usize, side, line.text.len())
            .map(|r| (r, self.tsel.color));
        let (text, layout) = line_text(line, spans, selected, fg, theme);
        let target = TextTarget {
            line: ix as usize,
            side,
            layout,
        };
        let text = self
            .tsel
            .bind_text(text_cell(text, fg, self.flow).bg(bg), target.clone());
        targets.push(target);
        el.child(marker_cell(marker, marker_color, Some(bg)))
            .child(text)
    }
}
