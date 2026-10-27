//! Main pane while a stash is selected: header, file list and read-only diff.

use gd_ui::vdivider;
use gpui::{AnyElement, Context, Entity, IntoElement, SharedString, div, prelude::*, px};

use crate::components::FileList;
use super::stash_header::StashHeader;
use crate::repo::RepoModel;
use crate::features::diff::DiffPane;
use crate::workspace::{NavList, Workspace, nav_focus};

impl Workspace {
    /// The diff renderer needs `&mut Workspace` (scroll state), so the detail
    /// pane is a method that composes the stateless pieces around it.
    pub fn render_stash_detail(&mut self, repo: &Entity<RepoModel>, cx: &mut Context<Self>) -> AnyElement {
        let r = repo.read(cx);
        let Some(stash) = r.stashes.selected_stash().cloned() else {
            return div().flex_1().into_any_element();
        };
        let st = &r.stashes;
        let here = stash.branch.is_some() && stash.branch.as_deref() == r.current_branch();
        let file_key = st.detail.path.clone().unwrap_or_default();
        let (diff, hl, n_files) = (st.detail.diff.clone(), st.detail.hl.clone(), st.detail.files.len());
        let diff_key = SharedString::from(format!("stash-diff-{}-{file_key}", stash.sha));
        let header = StashHeader { stash, n_files, here, repo: repo.clone(), ws: cx.entity() };
        let file_list = nav_focus(div(), NavList::Files(DiffPane::Stash), cx.entity())
            .flex()
            .flex_col()
            .flex_none()
            .w(px(260.0))
            .py(px(4.0))
            .child(FileList { repo: repo.clone(), pane: DiffPane::Stash });
        let diff = self.render_diff(diff, hl, diff_key, DiffPane::Stash, repo, cx);
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
