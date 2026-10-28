//! Main pane of a commit, a stash or a pull request: header, file list and diff.

use gd_ui::vdivider;
use gpui::{AnyElement, Context, Entity, IntoElement, SharedString, div, prelude::*, px};

use super::FileList;
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
            .py(px(4.0))
            .child(FileList { repo: repo.clone(), pane });
        let diff = self.render_diff(diff, hl, key, pane, repo, cx);
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
