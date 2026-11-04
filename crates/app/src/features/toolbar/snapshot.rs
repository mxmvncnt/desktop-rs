//! Snapshot view: the remote button's dropdown (needs a repo ahead of or behind its upstream).

use gpui::{Context, Window, px};

use crate::snapshot::View;
use crate::workspace::Workspace;

pub fn views() -> &'static [View] {
    &[View {
        name: "remote-menu",
        steps: &[menu],
    }]
}

fn menu(ws: &mut Workspace, _: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.open_remote_menu(px(640.0), window, cx);
}
