//! Main pane of the History tab: commit header, file list and diff.

use gpui::{AnyElement, Context, Entity, IntoElement, div, prelude::*};

use super::commit_header::CommitHeader;
use super::multi_select::MultiSelectPane;
use crate::features::diff::{DiffOptionsButton, DiffPane};
use crate::repo::RepoModel;
use crate::workspace::Workspace;

impl Workspace {
    pub fn render_history_main(&mut self, repo: &Entity<RepoModel>, cx: &mut Context<Self>) -> AnyElement {
        let r = repo.read(cx);
        if r.multi_indices().len() >= 2 {
            return MultiSelectPane { repo: repo.clone() }.into_any_element();
        }
        // Show the loaded commit (not the just-clicked one) so header, files
        // and diff change together instead of in separate steps.
        let detail = &r.commit_detail;
        let shown = detail.base.as_ref().and_then(|sha| r.commits.iter().find(|c| &c.sha == sha));
        let Some(commit) = shown.or_else(|| r.selected_commit.and_then(|ix| r.commits.get(ix))).cloned() else {
            return div().flex_1().into_any_element();
        };
        let header = CommitHeader::new(commit, detail.files.len(), DiffOptionsButton {
            ws: cx.entity(),
        }, self.commit_msg.height, cx.entity());
        self.render_file_diff_pane(header, repo, DiffPane::Commit, cx)
    }
}
