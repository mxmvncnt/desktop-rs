//! Workspace-side menu handlers: open/close the context menu and the discard
//! dialog.

use gd_git::FileStatus;
use gpui::{Context, Pixels, Point, Window};

use super::menu_rows::{file_rows, header_rows};
use super::state::ConfirmDialog;
use crate::components::Menu;
use crate::workspace::{Overlay, Workspace};

impl Workspace {
    pub fn open_menu(&mut self, menu: Menu, window: &mut Window, cx: &mut Context<Self>) {
        self.open_overlay(Overlay::Menu(menu), cx);
        // Focus the root so up/down/Enter/Esc reach us.
        self.focus(window, cx);
    }

    /// Open the menu for list row `ix` (resolved against the live model).
    pub fn open_file_menu(&mut self, ix: usize, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(change) = self.repo.as_ref().and_then(|r| r.read(cx).changes().get(ix).cloned()) else {
            return;
        };
        if change.status == FileStatus::Conflicted {
            return self.open_conflict_menu(change, position, window, cx);
        }
        let menu = Menu::new(position, file_rows(&change), move |ws, action, w, cx| {
            ws.run_menu_action(action, Some(change.clone()), w, cx)
        });
        self.open_menu(menu, window, cx);
    }

    pub fn open_header_menu(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if self.repo.as_ref().is_some_and(|r| !r.read(cx).changes().is_empty()) {
            let menu = Menu::new(position, header_rows(), |ws, action, w, cx| ws.run_menu_action(action, None, w, cx));
            self.open_menu(menu, window, cx);
        }
    }

    pub fn confirm_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(Overlay::Discard(dialog)) = self.overlays.pop_if(|o| matches!(o, Overlay::Discard(_))) else { return };
        cx.notify();
        let Some(repo) = self.repo.clone() else { return };
        let changes = match dialog {
            ConfirmDialog::DiscardFile(c) => vec![c],
            ConfirmDialog::DiscardAll(all) => all,
        };
        repo.update(cx, |r, cx| r.discard(changes, cx));
    }
}
