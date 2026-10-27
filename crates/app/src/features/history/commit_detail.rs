//! Main pane of the History tab: commit header, file list and diff.

use gd_ui::vdivider;
use gpui::{AnyElement, Context, Entity, IntoElement, SharedString, div, prelude::*, px};

use crate::components::{FileList, FileSource};
use super::commit_header::CommitHeader;
use super::multi_select::MultiSelectPane;
use crate::features::diff::{DiffOptionsButton, DiffPane};
use crate::repo::RepoModel;
use crate::workspace::{NavList, Workspace, nav_focus};

impl Workspace {
    /// The diff renderer needs `&mut Workspace` (scroll state), so the detail
    /// pane is a method that composes the stateless pieces around it.
    pub fn render_history_main(&mut self, repo: &Entity<RepoModel>, cx: &mut Context<Self>) -> AnyElement {
        let r = repo.read(cx);
        if r.multi_indices().len() >= 2 {
            return MultiSelectPane { repo: repo.clone() }.into_any_element();
        }
        // Show the loaded commit (not the just-clicked one) so header, files
        // and diff change together instead of in separate steps.
        let shown = r.commit_shown.as_ref().and_then(|s| r.commits.iter().find(|c| c.sha == s.sha));
        let Some(commit) = shown.or_else(|| r.selected_commit.and_then(|ix| r.commits.get(ix))).cloned() else {
            return div().flex_1().into_any_element();
        };
        let file_key = r.commit_shown.as_ref().and_then(|s| s.path.clone()).unwrap_or_default();
        let (diff, hl, n_files) = (r.commit_diff.clone(), r.commit_hl.clone(), r.commit_files.len());
        let diff_key = SharedString::from(format!("commit-diff-{}-{file_key}", commit.sha));
        let header = CommitHeader::new(commit, n_files, DiffOptionsButton {
            ws: cx.entity(),
        }, self.commit_msg.height, cx.entity());
        let file_list = nav_focus(div(), NavList::CommitFiles, cx.entity())
            .flex()
            .flex_col()
            .flex_none()
            .w(px(260.0))
            .py(px(4.0))
            .child(FileList {
                repo: repo.clone(),
                source: FileSource::Commit,
                scroll: self.commit_files_scroll.clone(),
            });
        let diff = self.render_diff(diff, hl, diff_key, DiffPane::History(repo.clone()), cx);
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
