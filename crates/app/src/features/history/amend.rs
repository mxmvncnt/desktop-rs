//! The commit box's "amend" mode (Amend from the commit menu).

use gd_git::Commit;
use gpui::{Context, Focusable, Window};

use crate::repo::Tab;
use crate::workspace::Workspace;

impl Workspace {
    pub fn amending(&self, cx: &gpui::App) -> bool {
        match (&self.history_ui.amend, &self.repo) {
            (Some(root), Some(r)) => root == r.read(cx).repo.root(),
            _ => false,
        }
    }

    /// Put the HEAD commit's message in the commit box and switch it to amend mode.
    pub fn start_amend(&mut self, commit: Commit, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else { return };
        let root = repo.read(cx).repo.root().to_path_buf();
        self.history_ui.amend = Some(root);
        self.set_commit_message(&commit.summary, &commit.body, cx);
        repo.update(cx, |r, cx| r.set_tab(Tab::Changes, cx));
        window.focus(&self.message.focus_handle(cx), cx);
        cx.notify();
    }

    pub fn stop_amend(&mut self, cx: &mut Context<Self>) {
        if self.history_ui.amend.take().is_some() {
            self.clear_commit_box(cx);
            cx.notify();
        }
    }

    /// Commit button while amending. `true` if it handled the commit.
    pub fn try_amend(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.amending(cx) {
            return false;
        }
        let Some(repo) = self.repo.clone() else { return true };
        let (summary, description) = self.commit_message(cx);
        if summary.trim().is_empty() || repo.read(cx).history_blocked() {
            return true;
        }
        repo.update(cx, |r, cx| r.amend(summary, description, cx));
        self.history_ui.amend = None;
        self.clear_commit_box(cx);
        self.focus(window, cx);
        true
    }

    fn clear_commit_box(&mut self, cx: &mut Context<Self>) {
        self.message.update(cx, |s, cx| s.emplace("", cx));
    }
}
