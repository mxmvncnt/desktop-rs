//! Confirming a history dialog: validate the inputs, then call the model.

use gpui::{Context, Entity, Window};

use super::menu_actions::reset_dialog;
use super::state::{HistoryAction, HistoryDialog};
use crate::utils::text::sanitize_branch_name;
use crate::repo::RepoModel;
use crate::workspace::{Overlay, Workspace};

/// Squash message typed in the dialog (both parts trimmed).
struct SquashMessage {
    summary: String,
    description: String,
}

impl Workspace {
    pub fn hist_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else { return };
        let name = sanitize_branch_name(self.history_ui.name.read(cx).as_str());
        let message = self.squash_message_input(cx);
        // Validate before consuming the dialog.
        match self.overlays.last() {
            Some(Overlay::History(HistoryDialog::CreateBranch(_) | HistoryDialog::CreateTag(_))) if name.is_empty() => return,
            Some(Overlay::History(HistoryDialog::Squash { .. } | HistoryDialog::Reword(_))) if message.summary.is_empty() => return,
            Some(Overlay::History(_)) => {}
            _ => return,
        }
        let Some(Overlay::History(dialog)) = self.overlays.pop() else { return };
        self.focus(window, cx);
        cx.notify();
        match dialog {
            HistoryDialog::PushedRewrite { action, commit } => self.confirm_rewrite(action, commit, &repo, window, cx),
            HistoryDialog::AmendPushed(c) => self.start_amend(c, window, cx),
            HistoryDialog::Reword(c) => {
                repo.update(cx, |r, cx| r.reword(c.sha, gd_git::message(&message.summary, &message.description), cx));
            }
            HistoryDialog::Squash { commits, .. } => {
                let shas = commits.into_iter().map(|c| c.sha).collect();
                repo.update(cx, |r, cx| r.squash(shas, gd_git::message(&message.summary, &message.description), cx));
            }
            other => confirm_simple(other, name, &repo, cx),
        }
    }

    fn squash_message_input(&self, cx: &gpui::App) -> SquashMessage {
        let read = |e: &Entity<gpui_elements::editable_text::EditableTextState>| e.read(cx).as_str().trim().to_string();
        SquashMessage { summary: read(&self.history_ui.msg_summary), description: read(&self.history_ui.msg_desc) }
    }

    fn confirm_rewrite(
        &mut self,
        action: HistoryAction,
        commit: gd_git::Commit,
        repo: &Entity<RepoModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            HistoryAction::Undo => repo.update(cx, |r, cx| r.undo_head_commit(cx)),
            HistoryAction::Reword => self.open_reword(commit, window, cx),
            HistoryAction::MoveUp => repo.update(cx, |r, cx| r.move_commit(commit.sha, true, cx)),
            HistoryAction::MoveDown => repo.update(cx, |r, cx| r.move_commit(commit.sha, false, cx)),
            // Reset still asks for the soft/mixed mode next.
            HistoryAction::Reset => self.open_overlay(Overlay::History(reset_dialog(commit)), cx),
            _ => {}
        }
    }
}

/// Dialogs that map straight onto one model call.
fn confirm_simple(dialog: HistoryDialog, name: String, repo: &Entity<RepoModel>, cx: &mut Context<Workspace>) {
    match dialog {
        HistoryDialog::Reset { commit, mode } => repo.update(cx, |r, cx| r.reset_to_commit(commit.sha, mode, cx)),
        HistoryDialog::Checkout(c) => repo.update(cx, |r, cx| r.checkout_commit(c.sha, cx)),
        HistoryDialog::CreateBranch(c) => repo.update(cx, |r, cx| r.create_branch_at(name, c.sha, cx)),
        HistoryDialog::CreateTag(c) => repo.update(cx, |r, cx| r.create_tag(name, c.sha, cx)),
        HistoryDialog::DeleteTag(t) => repo.update(cx, |r, cx| r.delete_tag(t, cx)),
        HistoryDialog::CherryPick { commits, target } => {
            if let Some(branch) = target.branches.get(target.selected).cloned() {
                let shas = commits.into_iter().map(|c| c.sha).collect();
                repo.update(cx, |r, cx| r.cherry_pick(shas, branch, cx));
            }
        }
        _ => {}
    }
}
