//! `impl Workspace`: thin forwards from UI commands to the repo model.

use gpui::{Context, Window};

use super::state::Workspace;
use crate::repo::{RemoteOp, RepoModel, Tab};

impl Workspace {
    /// Run `f` on the open repository, if any.
    pub(crate) fn with_repo(&self, cx: &mut Context<Self>, f: impl FnOnce(&mut RepoModel, &mut Context<RepoModel>)) {
        if let Some(repo) = &self.repo {
            repo.update(cx, f);
        }
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.with_repo(cx, |r, cx| r.refresh(cx));
    }

    pub(super) fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.try_amend(window, cx) {
            return;
        }
        let Some(repo) = self.repo.clone() else { return };
        let (mut summary, description) = self.commit_box.read(cx).text(cx);
        if !repo.read(cx).has_included() {
            return;
        }
        if summary.trim().is_empty() {
            match repo.read(cx).default_summary(false) {
                Some(d) => summary = d,
                None => return,
            }
        }
        repo.update(cx, |r, cx| r.commit(summary, description, cx));
        self.commit_box.update(cx, |b, cx| b.clear(cx));
        self.focus(window, cx);
    }

    pub fn undo_commit(&mut self, cx: &mut Context<Self>) {
        self.with_repo(cx, |r, cx| r.undo_last_commit(cx));
    }

    pub(crate) fn remote_action(&mut self, op: RemoteOp, cx: &mut Context<Self>) {
        let rewritten = self.repo.as_ref().is_some_and(|r| r.read(cx).work.status.as_ref().is_some_and(|s| s.rewritten));
        if op == RemoteOp::ForcePush || (op == RemoteOp::Push && rewritten) {
            return self.open_force_push(cx);
        }
        self.with_repo(cx, |r, cx| r.remote(op, cx));
    }

    pub(super) fn set_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.with_repo(cx, |r, cx| r.set_tab(tab, cx));
    }
}
