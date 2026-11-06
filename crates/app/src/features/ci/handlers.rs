//! `impl Workspace`: opening, closing and re-syncing the CI page.

use gpui::Context;

use super::CiEvent;
use crate::workspace::Workspace;

impl Workspace {
    /// The header button and Ctrl+Shift+I.
    pub fn toggle_ci(&mut self, cx: &mut Context<Self>) {
        if self.ci.read(cx).open {
            self.ci.update(cx, |ci, cx| ci.close(cx));
        } else {
            self.overlays.clear();
            self.pulls.update(cx, |p, cx| p.close(cx));
            self.sync_ci(cx);
        }
        cx.notify();
    }

    /// Open the page for the current repository, looking up its account again.
    pub fn sync_ci(&mut self, cx: &mut Context<Self>) {
        let Some(repo) = &self.repo else { return };
        let (repo, branch) = {
            let r = repo.read(cx);
            (r.repo.clone(), r.current_branch().map(str::to_string))
        };
        let (accounts, picks) = {
            let a = self.accounts.read(cx);
            (a.accounts.clone(), a.repo_accounts.clone())
        };
        self.ci
            .update(cx, |ci, cx| ci.show(repo, branch, accounts, picks, cx));
    }

    /// Esc: one screen back, then close the page. `false` when it is not open.
    pub fn back_ci(&mut self, cx: &mut Context<Self>) -> bool {
        let open = self.ci.read(cx).open;
        if open {
            self.ci.update(cx, |ci, cx| ci.back(cx));
        }
        open
    }

    /// Another repository: its branches and authors are not the ones typed.
    pub(crate) fn clear_ci_filters(&mut self, cx: &mut Context<Self>) {
        for combo in [&self.ci_inputs.branch, &self.ci_inputs.user] {
            combo.update(cx, |c, cx| c.set_value("", cx));
        }
    }

    /// The mouse Forward button: a screen of the CI page that Back left.
    pub fn forward_ci(&mut self, cx: &mut Context<Self>) {
        if self.ci.read(cx).open {
            self.ci.update(cx, |ci, cx| ci.forward(cx));
        }
    }

    pub(crate) fn on_ci_event(&mut self, ev: &CiEvent, cx: &mut Context<Self>) {
        match ev {
            CiEvent::Notice(message) => self.show_notice(message.clone(), cx),
            CiEvent::Error(message) => self.show_error(message.clone(), cx),
        }
    }
}
