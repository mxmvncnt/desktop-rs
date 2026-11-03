//! Workspace side: open the modal and the repository button's context menu.

use gpui::{Context, Pixels, Point, Window};

use super::state::RepoSettingsPage;
use crate::components::{Menu, MenuRow};
use crate::workspace::{Overlay, Workspace};

impl Workspace {
    pub fn open_repo_settings(&mut self, cx: &mut Context<Self>) {
        let Some(model) = self.repo.clone() else {
            return;
        };
        self.open_overlay(Overlay::RepoSettings, cx);
        self.repo_settings.update(cx, |s, cx| s.open(model, cx));
        self.load_repo_identity(cx);
    }

    /// The repository's account pick (from the CI page).
    pub fn open_repo_account_settings(&mut self, cx: &mut Context<Self>) {
        self.open_repo_settings(cx);
        self.set_repo_settings_page(RepoSettingsPage::Account, cx);
    }

    pub(crate) fn set_repo_settings_page(
        &mut self,
        page: RepoSettingsPage,
        cx: &mut Context<Self>,
    ) {
        self.repo_settings.update(cx, |s, _| {
            (s.page, s.edit, s.error, s.notice) = (page, None, None, None)
        });
        self.load_repo_identity(cx);
        cx.notify();
    }

    /// The identity in effect and the remote host (account pick) depend on the repository config.
    fn load_repo_identity(&mut self, cx: &mut Context<Self>) {
        let repo = self.repo.as_ref().map(|r| r.read(cx).repo.clone());
        self.accounts.update(cx, |a, cx| a.load_identity(repo, cx));
    }

    /// Right-click on the repository button.
    pub fn open_repo_menu(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.repo.is_none() {
            return;
        }
        let rows = vec![MenuRow::item(
            (),
            "Repository settings…",
            gd_ui::Icon::Settings,
        )];
        let menu = Menu::new(position, rows, |ws, (), _, cx| ws.open_repo_settings(cx));
        self.open_menu(menu, window, cx);
    }
}
