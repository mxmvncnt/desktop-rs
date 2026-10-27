//! Snapshot views: the pull-request preview pane and its base-branch menu.

use gpui::{Context, Window};

use crate::snapshot::View;
use crate::workspace::Workspace;

pub fn views() -> &'static [View] {
    &[View { name: "pr-preview*", steps: &[open_preview] }]
}

fn open_preview(ws: &mut Workspace, view: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    let Some(r) = &ws.repo else { return };
    r.update(cx, |r, cx| {
        r.open_pr_preview(cx);
        r.pr.base_menu = view == "pr-preview-menu";
    });
}
