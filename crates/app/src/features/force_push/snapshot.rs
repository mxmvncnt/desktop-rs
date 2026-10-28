//! Snapshot view: the force push modal (needs a repo with a rewritten, pushed history).

use gpui::{Context, Window};

use crate::snapshot::View;
use crate::workspace::Workspace;

pub fn views() -> &'static [View] {
    &[View { name: "force-push", steps: &[open_preview] }]
}

fn open_preview(ws: &mut Workspace, _: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    ws.open_force_push(cx);
}
