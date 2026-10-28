//! Main pane of the Changes tab. An `impl Workspace` method because the diff
//! renderer owns scroll state (`&mut Workspace`); the pieces around it are
//! stateless components.

use gpui::{AnyElement, Context, Entity, IntoElement, ParentElement, SharedString, Styled, div};

use super::file_header::FileHeader;
use super::no_changes::NoChanges;
use crate::features::diff::DiffPane;
use crate::features::merge::ConflictHint;
use crate::repo::RepoModel;
use crate::workspace::Workspace;
use gd_git::FileStatus;

impl Workspace {
    pub fn render_changes_main(&mut self, repo: &Entity<RepoModel>, cx: &mut Context<Self>) -> AnyElement {
        let r = repo.read(cx);
        if r.stashes.viewing {
            return self.render_stash_detail(repo, cx);
        }
        let Some(change) = r.selected_change_file().cloned() else {
            return NoChanges::new(r).into_any_element();
        };
        let diff = r.work.detail.diff.clone();
        let hl = r.work.detail.hl.clone();
        let hint = (change.status == FileStatus::Conflicted)
            .then(|| ConflictHint::new(&change, diff.as_ref(), repo.clone(), cx));
        let header = FileHeader::new(&change, cx.entity());
        // Key on the loaded diff's path (not the clicked one) so the fade runs
        // when the content swaps, not on the previous file's diff.
        let shown = r.work.diff_key.as_ref().map_or(change.path.as_str(), |(p, _)| p.as_str());
        let diff_id = SharedString::from(format!("change-diff-{shown}"));
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .child(header)
            .children(hint)
            .child(self.render_diff(diff, hl, diff_id, DiffPane::Changes, repo, cx))
            .into_any_element()
    }
}
