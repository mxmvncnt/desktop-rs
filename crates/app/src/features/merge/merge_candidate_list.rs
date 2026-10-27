//! Virtualized list of branches the merge dialog can merge from.

use gd_git::{Branch, BranchKind};
use gd_ui::{Icon, empty_state};
use gpui::{
    App, Entity, IntoElement, ParentElement, RenderOnce, Styled,
    UniformListScrollHandle, Window, uniform_list,
};

use crate::components::BranchRow;
use crate::workspace::Workspace;

pub const ROW_HEIGHT: f32 = 30.0;

#[derive(IntoElement)]
pub struct MergeCandidateList {
    branches: Vec<Branch>,
    selected: usize,
    scroll: UniformListScrollHandle,
    ws: Entity<Workspace>,
}

impl MergeCandidateList {
    pub fn new(branches: Vec<Branch>, selected: usize, scroll: UniformListScrollHandle, ws: Entity<Workspace>) -> Self {
        Self { branches, selected, scroll, ws }
    }
}

impl RenderOnce for MergeCandidateList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let n = self.branches.len();
        if n == 0 {
            return empty_state("No matching branches", cx).h_full().into_any_element();
        }
        let (rows, ws, selected) = (self.branches, self.ws, self.selected);
        uniform_list("merge-branches", n, move |range, _w, cx| {
            range
                .filter_map(|i| Some(row(i, rows.get(i)?, i == selected, ws.clone(), cx)))
                .collect()
        })
        .track_scroll(&self.scroll)
        .size_full()
        .into_any_element()
    }
}

fn row(i: usize, b: &Branch, selected: bool, ws: Entity<Workspace>, cx: &App) -> impl IntoElement + use<> {
    let kind_icon = if b.kind == BranchKind::Remote { Icon::Cloud } else { Icon::GitBranch };
    BranchRow::new("merge-branch", i, b.name.clone(), move |_, cx| ws.update(cx, |this, cx| this.merge_select(i, cx)))
        .icon(kind_icon)
        .selected(selected)
        .bright(selected)
        .child(BranchRow::tip_time(b.tip_time, cx))
}
