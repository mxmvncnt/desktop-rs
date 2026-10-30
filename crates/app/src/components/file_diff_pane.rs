//! Main pane of a commit, a stash or a pull request: header, file list and diff.

use gd_ui::{Theme, vdivider};
use gpui::{AnyElement, Context, Entity, FontWeight, IntoElement, SharedString, div, prelude::*, px};

use super::{DiffStat, FileList, drag_selection, selectable_line};
use crate::features::diff::DiffPane;
use crate::repo::RepoModel;
use crate::workspace::{NavList, Workspace, nav_focus};

impl Workspace {
    /// The diff renderer needs `&mut Workspace` (scroll state), so the pane is
    /// a method that composes the stateless pieces around it. `pane` is
    /// `Commit`, `Stash` or `Pr`.
    pub fn render_file_diff_pane(
        &mut self,
        header: impl IntoElement,
        repo: &Entity<RepoModel>,
        pane: DiffPane,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let detail = repo.read(cx).detail(pane);
        let key = SharedString::from(format!(
            "{pane:?}-diff-{}-{}",
            detail.base.as_deref().unwrap_or_default(),
            detail.path.as_deref().unwrap_or_default()
        ));
        let (diff, hl) = (detail.diff.clone(), detail.hl.clone());
        let file_list = nav_focus(div(), NavList::Files(pane), cx.entity())
            .flex()
            .flex_col()
            .flex_none()
            .w(px(260.0))
            .child(FileList { repo: repo.clone(), pane });
        let stat = diff.as_ref().map(|d| DiffStat { added: d.added, deleted: d.deleted });
        let theme = Theme::of(cx).clone();
        let texts = [detail.path.clone().unwrap_or_default(), String::new()];
        let sel = self.commit_sel.on(&texts);
        let (path, path_layout) =
            selectable_line(&cx.entity(), &texts, 0, sel, theme.text_muted, FontWeight::NORMAL, &theme);
        let diff = self.render_diff(diff, hl, key, pane, repo, cx);
        let diff = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.0))
                    .h(px(32.0))
                    .px(px(16.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .on_mouse_move(drag_selection(cx.entity(), vec![path_layout]))
                    .child(div().flex_1().min_w_0().truncate().text_size(px(12.5)).text_color(theme.text_muted).child(path))
                    .children(stat),
            )
            .child(diff);
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .child(header)
            .child(div().flex().flex_1().min_h_0().child(file_list).child(vdivider(cx)).child(diff))
            .into_any_element()
    }
}
