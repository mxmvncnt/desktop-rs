//! Snapshot views: merge / squash / rebase dialogs.

use gpui::{Context, Window};

use super::MergeMode;
use crate::snapshot::View;
use crate::workspace::Workspace;

pub fn views() -> &'static [View] {
    &[
        View { name: "merge-dialog", steps: &[merge_dialog] },
        View { name: "squash-dialog", steps: &[merge_dialog] },
        View { name: "rebase-dialog", steps: &[merge_dialog] },
    ]
}

fn merge_dialog(ws: &mut Workspace, view: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    let mode = match view {
        "squash-dialog" => MergeMode::Squash,
        "rebase-dialog" => MergeMode::Rebase,
        _ => MergeMode::Merge,
    };
    ws.open_merge_dialog(mode, None, window, cx);
}
