//! Main pane while the force push preview is open: header, file list and
//! read-only diff. An `impl Workspace` method because the diff renderer owns
//! scroll state (`&mut Workspace`).

use gd_ui::Theme;
use gpui::{AnyElement, Context, Entity, IntoElement, div, prelude::*};

use super::force_push_header::ForcePushHeader;
use crate::features::diff::DiffPane;
use crate::repo::RepoModel;
use crate::workspace::Workspace;

impl Workspace {
    pub fn render_force_push(&mut self, repo: &Entity<RepoModel>, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::of(cx).clone();
        let r = repo.read(cx);
        let header = ForcePushHeader { repo: repo.clone() };
        let text = match (&r.force_push.preview, r.force_push.loading) {
            (Some(p), _) if !p.files.is_empty() => return self.render_file_diff_pane(header, repo, DiffPane::ForcePush, cx),
            (Some(_), _) => "The file contents are identical: only the commit history changes.",
            (None, true) => "Comparing with the remote…",
            (None, false) => "Nothing to compare with the remote.",
        };
        let body = div().flex().flex_1().items_center().justify_center().text_color(theme.text_muted).child(text);
        div().flex().flex_col().flex_1().min_w_0().child(header).child(body).into_any_element()
    }

    /// Esc: leave the preview. False if it is not open.
    pub(crate) fn close_force_push_key(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(repo) = self.repo.clone().filter(|r| r.read(cx).force_push.open) else { return false };
        repo.update(cx, |r, cx| r.close_force_push(cx));
        true
    }
}
