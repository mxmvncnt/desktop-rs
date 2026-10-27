//! Arrow / Enter routing across every open menu and dialog. Esc is routed in
//! `workspace/actions.rs`.

use gpui::{Context, Window};

use crate::components::Menu;
use crate::workspace::Workspace;

impl Workspace {
    /// The open context menu (they never coexist; history's wins).
    fn open_menu_state(&mut self) -> Option<&mut Menu> {
        [&mut self.history_ui.menu, &mut self.merge_ui.menu, &mut self.branch_ui.menu, &mut self.menu]
            .into_iter()
            .flatten()
            .next()
    }

    pub fn menu_step(&mut self, delta: isize, cx: &mut Context<Self>) {
        if let Some(menu) = self.open_menu_state() {
            menu.step(delta);
            cx.notify();
        }
    }

    /// Enter: run the highlighted menu item, or confirm the open dialog.
    pub fn menu_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.stash_ui.dialog.is_some() {
            self.stash_confirm(cx);
            return;
        }
        if self.branch_ui_confirm(window, cx) {
            return;
        }
        if self.dialog.is_some() {
            self.confirm_dialog(cx);
            return;
        }
        if let Some(run) = self.open_menu_state().and_then(|m| m.confirm()) {
            run(self, window, cx);
        }
    }
}
