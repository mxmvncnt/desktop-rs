//! Snapshot views: file and header context menus, discard dialog.

use gpui::{Context, Window, point, px};

use super::ConfirmDialog;
use crate::snapshot::View;
use crate::workspace::{Overlay, Workspace};

pub fn views() -> &'static [View] {
    &[
        View { name: "context-menu", steps: &[|ws, _, w, cx| ws.open_file_menu(1, point(px(120.0), px(150.0)), w, cx)] },
        View { name: "conflict-menu", steps: &[|ws, _, w, cx| ws.open_file_menu(0, point(px(120.0), px(150.0)), w, cx)] },
        View { name: "header-menu", steps: &[|ws, _, w, cx| ws.open_header_menu(point(px(60.0), px(100.0)), w, cx)] },
        View { name: "discard-dialog", steps: &[discard_dialog] },
    ]
}

fn discard_dialog(ws: &mut Workspace, _: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    if let Some(c) = ws.repo.as_ref().and_then(|r| r.read(cx).changes().get(1).cloned()) {
        ws.open_overlay(Overlay::Discard(ConfirmDialog::File(c)), cx);
    }
}
