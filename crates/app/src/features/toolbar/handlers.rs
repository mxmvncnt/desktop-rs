//! Workspace side: the remote button's dropdown.

use gd_ui::{Icon, theme::TOOLBAR_HEIGHT};
use gpui::{Context, Pixels, Window, point, px};

use super::remote_button::RemoteButton;
use crate::components::{Menu, MenuRow};
use crate::repo::RemoteOp;
use crate::workspace::Workspace;

impl Workspace {
    /// Chevron beside the remote button: fetch while the button offers push / pull.
    /// The menu drops from the toolbar's bottom edge at `x`.
    pub fn open_remote_menu(&mut self, x: Pixels, window: &mut Window, cx: &mut Context<Self>) {
        let repo = self.repo.as_ref().map(|r| r.read(cx));
        let Some(title) = RemoteButton::for_repo(repo).fetch_title else {
            return;
        };
        let inset = window.client_inset().unwrap_or(px(0.0));
        let position = point(x, inset + px(TOOLBAR_HEIGHT));
        let rows = vec![MenuRow::item(RemoteOp::Fetch, title, Icon::Refresh)];
        let menu = Menu::new(position, rows, |ws, op, _, cx| ws.remote_action(op, cx));
        self.open_menu(menu, window, cx);
    }
}
