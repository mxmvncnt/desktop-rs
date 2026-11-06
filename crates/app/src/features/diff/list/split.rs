//! The virtualized list of a split diff: one list whose rows each hold both
//! halves, so both sides always scroll together vertically.

use std::rc::Rc;
use std::sync::Arc;

use gd_ui::Theme;
use gpui::{
    App, IntoElement, ListState, RenderOnce, SharedString, UniformListScrollHandle, Window, px,
};

use super::rows::rows_list;
use super::split_model::{NONE, SplitDiff};
use super::split_rows::SplitRows;
use super::text_flow::{MARKER_W, TextFlow};
use crate::features::diff::expand::Expander;
use crate::features::diff::gutter::RowSelect;
use crate::features::diff::gutter::number_cell_w;
use crate::features::diff::highlight::DiffHighlight;
use crate::features::diff::notes::RowNotes;
use crate::features::diff::text_selection::TextSelect;

#[derive(IntoElement)]
pub struct SplitList {
    pub split: Arc<SplitDiff>,
    pub hl: Option<Arc<DiffHighlight>>,
    pub select: Option<RowSelect>,
    pub exp: Option<Expander>,
    /// Whether a trailing "expand down" row follows the last hunk.
    pub trailing: bool,
    pub scroll: UniformListScrollHandle,
    pub wrap: Option<ListState>,
    pub flow: TextFlow,
    pub tsel: TextSelect,
    pub notes: Option<Rc<RowNotes>>,
    pub mono: SharedString,
}

impl RenderOnce for SplitList {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Self {
            split,
            hl,
            select,
            exp,
            trailing,
            scroll,
            wrap,
            flow,
            tsel,
            notes,
            mono,
        } = self;
        let n = split.rows.len() + usize::from(trailing);
        let num_w = px(split.gutter_digits as f32 * 7.5 + 16.0);
        let exp_w = number_cell_w(num_w, 1.0, select.is_some()) + px(MARKER_W);
        rows_list(
            "diff-split",
            n,
            wrap,
            &scroll,
            mono,
            move |ix, theme: &Theme, cx| {
                let sel = select.as_ref().map(|s| (s, s.repo.read(cx)));
                let row = SplitRows {
                    split: &split,
                    hl: hl.as_deref(),
                    sel,
                    exp: exp.as_ref(),
                    num_w,
                    exp_w,
                    flow,
                    tsel: &tsel,
                    notes: notes.as_deref(),
                    theme,
                }
                .row(ix);
                match (&notes, split.rows.get(ix)) {
                    (Some(n), Some(r)) => {
                        // A context row is one line on both sides.
                        let lines: Vec<usize> = [r.left, r.right]
                            .into_iter()
                            .filter(|&l| l != NONE)
                            .map(|l| l as usize)
                            .fold(Vec::new(), |mut v, l| {
                                if !v.contains(&l) {
                                    v.push(l);
                                }
                                v
                            });
                        n.wrap(row, &lines, cx)
                    }
                    _ => row,
                }
            },
        )
    }
}
