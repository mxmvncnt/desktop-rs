//! Workspace-side handlers of the branch menu and the rename / delete /
//! create-from dialogs: open, keyboard navigation, Enter, Esc, confirm.

use gd_git::{Branch, BranchKind};
use gpui::{ClipboardItem, Context, Focusable, Pixels, Point, Window};

use super::branch_menu_rows::branch_menu_rows;
use super::state::{BranchAction, BranchDialog, RemoteBranch};
use crate::components::Menu;
use crate::features::merge::MergeMode;
use crate::utils::text::sanitize_branch_name;
use crate::workspace::Workspace;

impl Workspace {
    pub fn open_branch_menu(&mut self, branch: Branch, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        self.merge_ui.menu = None;
        let rows = branch_menu_rows(&branch, self.paused(cx));
        self.branch_ui.menu = Some(Menu::new(position, rows, move |ws, action, w, cx| {
            ws.run_branch_action(action, branch.clone(), w, cx)
        }));
        self.branch_ui.menu_gen += 1;
        self.focus(window, cx);
        cx.notify();
    }

    pub(crate) fn close_branch_menu(&mut self) {
        self.branch_ui.menu = None;
    }

    /// Drop the menu and dialog without touching focus (another overlay opens).
    pub(crate) fn close_branch_overlays(&mut self) {
        self.branch_ui.menu = None;
        self.branch_ui.dialog = None;
    }

    /// Esc: close the menu, else the dialog. Returns whether one was open.
    pub fn close_branch_ui(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let open = self.branch_ui.menu.is_some() || self.branch_ui.dialog.is_some();
        if self.branch_ui.menu.take().is_none() {
            self.branch_ui.dialog = None;
            self.focus(window, cx);
        }
        if open {
            cx.notify();
        }
        open
    }

    /// Enter while the delete dialog is open. Returns whether handled.
    pub fn branch_ui_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let delete = matches!(self.branch_ui.dialog, Some(BranchDialog::Delete { .. }));
        if delete {
            self.branch_dialog_confirm(window, cx);
        }
        delete
    }

    pub(crate) fn run_branch_action(&mut self, action: BranchAction, b: Branch, window: &mut Window, cx: &mut Context<Self>) {
        self.branch_ui.menu = None;
        cx.notify();
        match action {
            BranchAction::Merge => self.open_merge_dialog(MergeMode::Merge, Some(b.name), window, cx),
            BranchAction::Rebase => self.open_merge_dialog(MergeMode::Rebase, Some(b.name), window, cx),
            BranchAction::CopyName => cx.write_to_clipboard(ClipboardItem::new_string(b.name)),
            BranchAction::CreateFrom => {
                self.open_branch_dialog(BranchDialog::CreateFrom { base: b.name }, "", window, cx)
            }
            BranchAction::Rename => {
                let old = b.name.clone();
                self.open_branch_dialog(BranchDialog::Rename { old: old.clone() }, &old, window, cx)
            }
            BranchAction::Delete => self.open_delete_dialog(b, window, cx),
        }
    }

    fn open_delete_dialog(&mut self, branch: Branch, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else { return };
        // Offer deleting the upstream too when it still exists.
        let remote = (branch.kind == BranchKind::Local).then(|| branch.upstream.clone()).flatten().filter(|u| {
            repo.read(cx).branches.iter().any(|x| x.kind == BranchKind::Remote && &x.name == u)
        });
        let dialog = BranchDialog::Delete { branch, remote, also_remote: false };
        self.open_branch_dialog(dialog, "", window, cx)
    }

    fn open_branch_dialog(&mut self, dialog: BranchDialog, prefill: &str, window: &mut Window, cx: &mut Context<Self>) {
        let delete = matches!(dialog, BranchDialog::Delete { .. });
        self.popover = None;
        self.branch_ui.input.update(cx, |s, cx| s.emplace(prefill, cx));
        self.branch_ui.dialog = Some(dialog);
        if delete {
            self.focus(window, cx);
        } else {
            window.focus(&self.branch_ui.input.focus_handle(cx), cx);
        }
        cx.notify();
    }

    pub(super) fn toggle_also_remote(&mut self, cx: &mut Context<Self>) {
        if let Some(BranchDialog::Delete { also_remote, .. }) = &mut self.branch_ui.dialog {
            *also_remote = !*also_remote;
            cx.notify();
        }
    }

    /// Name typed into the rename / create-from input, sanitized.
    pub(super) fn typed_branch_name(&self, cx: &gpui::App) -> String {
        sanitize_branch_name(self.branch_ui.input.read(cx).as_str())
    }

    pub(super) fn branch_dialog_valid(&self, cx: &gpui::App) -> bool {
        let name = self.typed_branch_name(cx);
        let exists = |n: &str| {
            self.repo
                .as_ref()
                .is_some_and(|r| r.read(cx).branches.iter().any(|b| b.kind == BranchKind::Local && b.name == n))
        };
        match &self.branch_ui.dialog {
            Some(BranchDialog::Rename { old }) => !name.is_empty() && &name != old && !exists(&name),
            Some(BranchDialog::CreateFrom { .. }) => !name.is_empty() && !exists(&name),
            Some(BranchDialog::Delete { .. }) => true,
            None => false,
        }
    }

    pub fn branch_dialog_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.branch_dialog_valid(cx) {
            return;
        }
        let name = self.typed_branch_name(cx);
        let Some(dialog) = self.branch_ui.dialog.take() else { return };
        self.focus(window, cx);
        cx.notify();
        let Some(repo) = self.repo.clone() else { return };
        repo.update(cx, |r, cx| match dialog {
            BranchDialog::Rename { old } => r.rename_branch(old, name, cx),
            BranchDialog::CreateFrom { base } => r.create_branch_from(name, base, cx),
            BranchDialog::Delete { branch, remote, also_remote } => {
                let target = delete_target(&branch, remote.as_deref(), also_remote);
                r.delete_branch(branch, target, cx)
            }
        });
    }
}

/// Which server branch to delete: the branch itself when it is a remote one,
/// else its upstream if the user ticked the box.
fn delete_target(branch: &Branch, upstream: Option<&str>, also_remote: bool) -> Option<RemoteBranch> {
    if branch.kind == BranchKind::Remote {
        RemoteBranch::parse(&branch.name)
    } else if also_remote {
        upstream.and_then(RemoteBranch::parse)
    } else {
        None
    }
}
