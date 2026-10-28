//! The commit box's "amend" mode (Amend from the commit menu); the state lives in `CommitBox`.

use gd_git::Commit;
use gpui::{Context, Window};

use crate::repo::Tab;
use crate::workspace::Workspace;

impl Workspace {
    /// Put the HEAD commit's message in the commit box and switch it to amend mode.
    pub fn start_amend(&mut self, commit: Commit, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else { return };
        self.commit_box.update(cx, |b, cx| b.start_amend(&commit, window, cx));
        repo.update(cx, |r, cx| r.set_tab(Tab::Changes, cx));
        cx.notify();
    }

    /// Commit button while amending. `true` if it handled the commit.
    pub fn try_amend(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.commit_box.read(cx).amending(cx) {
            return false;
        }
        let Some(repo) = self.repo.clone() else { return true };
        let (summary, description) = self.commit_box.read(cx).text(cx);
        if summary.trim().is_empty() || repo.read(cx).history_blocked() {
            return true;
        }
        repo.update(cx, |r, cx| r.amend(summary, description, cx));
        self.commit_box.update(cx, |b, cx| b.stop_amend(cx));
        self.focus(window, cx);
        true
    }
}
