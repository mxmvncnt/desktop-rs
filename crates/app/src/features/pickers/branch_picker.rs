//! Branch list popover.

use gd_git::BranchKind;
use gd_ui::{Button, Icon};
use gpui::{
    AnyElement, App, Entity, IntoElement, ParentElement, RenderOnce,
    Styled, UniformListScrollHandle, Window, div, px, uniform_list,
};
use gpui_elements::editable_text::EditableTextState;

use super::picker_shell::{PICKER_ROW, PickerShell};
use crate::features::merge::MergeMode;
use crate::repo::RepoModel;
use crate::components::BranchRow;
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct BranchPicker {
    /// Indices into `RepoModel::branches` matching the filter.
    pub items: Vec<usize>,
    pub selected: usize,
    pub repo: Entity<RepoModel>,
    pub filter: Entity<EditableTextState>,
    pub scroll: UniformListScrollHandle,
    pub ws: Entity<Workspace>,
}

impl BranchPicker {

    fn list(&self) -> AnyElement {
        let (items, selected) = (self.items.clone(), self.selected);
        let (repo, ws) = (self.repo.clone(), self.ws.clone());
        uniform_list("branch-list", self.items.len(), move |range, _w, cx| {
            let r = repo.read(cx);
            let rows: Vec<_> = range
                .filter_map(|i| Some((i, r.branches.get(*items.get(i)?)?.clone())))
                .collect();
            rows.into_iter().map(|(i, b)| branch_row(i, b, i == selected, &ws, cx)).collect()
        })
        .track_scroll(&self.scroll)
        .px(px(6.0))
        .size_full()
        .into_any_element()
    }

    fn footer(&self) -> impl IntoElement + use<> {
        let (new, merge) = (self.ws.clone(), self.ws.clone());
        div()
            .flex()
            .gap(px(6.0))
            .w_full()
            .child(
                Button::new("new-branch")
                    .icon(Icon::Plus)
                    .label("New branch")
                    .compact()
                    .on_click(move |_, w, cx| new.update(cx, |this, cx| this.open_new_branch_from_filter(w, cx))),
            )
            .child(
                Button::new("merge-into-current")
                    .icon(Icon::Fork)
                    .label("Merge into current branch…")
                    .compact()
                    .on_click(move |_, w, cx| {
                        merge.update(cx, |this, cx| this.open_merge_dialog(MergeMode::Merge, None, w, cx))
                    }),
            )
    }
}

impl RenderOnce for BranchPicker {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (list, footer) = (self.list(), self.footer());
        let n = self.items.len();
        PickerShell::new("Branches", "Filter branches", self.filter, n, list, footer, self.ws)
            .on_enter(|this, w, cx| this.enter_branch_picker(w, cx))
    }
}

fn branch_icon(b: &gd_git::Branch) -> Icon {
    if b.is_head {
        Icon::Check
    } else if b.kind == BranchKind::Remote {
        Icon::Cloud
    } else {
        Icon::GitBranch
    }
}

/// Hot row (virtualized): a plain fn, not a component.
fn branch_row(i: usize, b: gd_git::Branch, selected: bool, ws: &Entity<Workspace>, cx: &mut App) -> AnyElement {
    let (ws_menu, ws_pick) = (ws.clone(), ws.clone());
    let (b_menu, b_pick) = (b.clone(), b.clone());
    BranchRow::new("branch", i, b.name.clone(), move |w, cx| {
        let b = b_pick.clone();
        ws_pick.update(cx, |this, cx| this.pick_branch(b, w, cx))
    })
    .icon(branch_icon(&b))
    .height(PICKER_ROW)
    .selected(selected)
    .bright(b.is_head)
    .on_right_click(move |pos, w, cx| {
        let b = b_menu.clone();
        ws_menu.update(cx, |this, cx| this.open_branch_menu(b, pos, w, cx))
    })
    .child(BranchRow::tip_time(b.tip_time, cx))
    .into_any_element()
}
