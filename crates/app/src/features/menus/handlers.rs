//! Workspace-side menu handlers: open/close the context menu and the discard
//! dialog.

use gd_git::FileStatus;
use gpui::{Context, Pixels, Point, Window};

use gd_ui::Icon;

use super::menu_rows::{file_rows, header_rows};
use super::state::{ConfirmDialog, MenuAction};
use crate::components::{Menu, MenuRow, RowItem};
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

    /// Right-click on a diff gutter: offer to discard `rows` (one line, or a
    /// whole chunk when `chunk`) of the displayed working diff.
    pub fn open_discard_rows_menu(&mut self, rows: Vec<usize>, chunk: bool, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.as_ref().map(|r| r.read(cx)) else { return };
        let (Some(path), Some(diff)) = (repo.selected_path(), repo.work.detail.diff.clone()) else { return };
        let label = if chunk { "Discard chunk…" } else { "Discard line…" };
        let item = MenuRow::Item(RowItem::new(MenuAction::DiscardRows, label, Icon::Trash).danger());
        let menu = Menu::new(position, vec![item], move |ws, _, _, cx| {
            let dialog = ConfirmDialog::Rows { path: path.clone(), diff: diff.clone(), rows: rows.clone() };
            ws.open_overlay(Overlay::Discard(dialog), cx)
        });
        self.open_menu(menu, window, cx);
    }

    pub fn confirm_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(Overlay::Discard(dialog)) = self.overlays.pop_if(|o| matches!(o, Overlay::Discard(_))) else { return };
        cx.notify();
        let Some(repo) = self.repo.clone() else { return };
        match dialog {
            ConfirmDialog::File(c) => repo.update(cx, |r, cx| r.discard(vec![c], cx)),
            ConfirmDialog::All(all) => repo.update(cx, |r, cx| r.discard(all, cx)),
            ConfirmDialog::Rows { path, diff, rows } => repo.update(cx, |r, cx| r.discard_rows(path, diff, rows, cx)),
        }
    }
}
