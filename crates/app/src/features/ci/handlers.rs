//! `impl Workspace`: opening, closing and re-syncing the CI page.

use gpui::{App, Context, Entity, Focusable as _, Window};
use gpui_elements::editable_text::EditableTextState;

use super::CiEvent;
use super::filter_entries::{Entry, branch_entries, user_entries};
use super::state::Field;
use crate::workspace::Workspace;

impl Workspace {
    /// The header button and Ctrl+Shift+I.
    pub fn toggle_ci(&mut self, cx: &mut Context<Self>) {
        if self.ci.read(cx).open {
            self.ci.update(cx, |ci, cx| ci.close(cx));
        } else {
            self.overlays.clear();
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

    /// A filter input of the pipelines list changed.
    pub(crate) fn ci_text_edited(
        &mut self,
        field: Field,
        input: &Entity<EditableTextState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let typed = input.focus_handle(cx).is_focused(window);
        let text = input.read(cx).as_str().trim().to_string();
        self.ci
            .update(cx, |ci, cx| ci.text_edited(field, text, typed, cx));
    }

    /// A click outside the dropdown of a filter (or Esc): close it, and the text typed but not picked goes back to what the filter is.
    pub(crate) fn close_ci_menu(&mut self, cx: &mut Context<Self>) {
        let ci = self.ci.read(cx);
        if ci.menu.is_none() {
            return;
        }
        let (branch, user) = (ci.branch.clone(), ci.user.clone());
        self.ci.update(cx, |ci, cx| ci.close_menu(cx));
        self.ci_inputs
            .branch
            .update(cx, |s, cx| s.emplace(&branch, cx));
        self.ci_inputs.user.update(cx, |s, cx| s.emplace(&user, cx));
    }

    /// What the dropdown of `field` lists, and whether text narrows it; `None` while it is closed.
    pub(crate) fn ci_entries(&self, field: Field, cx: &App) -> Option<(Vec<Entry>, bool)> {
        let ci = self.ci.read(cx);
        if ci.menu != Some(field) {
            return None;
        }
        let input = match field {
            Field::Branch => &self.ci_inputs.branch,
            Field::User => &self.ci_inputs.user,
        };
        let typed = if ci.menu_typed {
            input.read(cx).as_str().trim().to_string()
        } else {
            String::new()
        };
        let entries = match field {
            Field::Branch => {
                let current = ci.ready().and_then(|t| t.branch.clone());
                self.repo
                    .as_ref()
                    .map(|r| branch_entries(&r.read(cx).branches, current.as_deref(), &typed))
                    .unwrap_or_default()
            }
            Field::User => user_entries(ci.known_users(), &typed),
        };
        Some((entries, !typed.is_empty()))
    }

    /// Another repository: its branches and authors are not the ones typed.
    pub(crate) fn clear_ci_filters(&mut self, cx: &mut Context<Self>) {
        for input in [&self.ci_inputs.branch, &self.ci_inputs.user] {
            input.update(cx, |s, cx| s.emplace("", cx));
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
