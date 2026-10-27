//! Main pane while a stash is selected: header, file list and read-only diff.

use gpui::{AnyElement, Context, Entity, IntoElement, div, prelude::*};

use super::stash_header::StashHeader;
use crate::features::diff::DiffPane;
use crate::repo::RepoModel;
use crate::workspace::Workspace;

impl Workspace {
    pub fn render_stash_detail(&mut self, repo: &Entity<RepoModel>, cx: &mut Context<Self>) -> AnyElement {
        let r = repo.read(cx);
        let Some(stash) = r.stashes.selected_stash().cloned() else {
            return div().flex_1().into_any_element();
        };
        let here = stash.branch.is_some() && stash.branch.as_deref() == r.current_branch();
        let n_files = r.stashes.detail.files.len();
        let header = StashHeader { stash, n_files, here, repo: repo.clone(), ws: cx.entity() };
        self.render_file_diff_pane(header, repo, DiffPane::Stash, cx)
    }
}
