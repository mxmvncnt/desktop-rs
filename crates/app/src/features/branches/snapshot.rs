//! Snapshot views: branch context menu and its rename / delete dialogs.

use gpui::{Context, Window, point, px};

use super::state::BranchAction;
use crate::snapshot::View;
use crate::workspace::{Popover, Workspace};

pub fn views() -> &'static [View] {
    &[
        View { name: "branch-menu", steps: &[branch_menu] },
        View { name: "branch-menu-current", steps: &[branch_menu] },
        View { name: "delete-dialog", steps: &[branch_menu] },
        View { name: "rename-dialog", steps: &[branch_menu] },
    ]
}

fn branch_menu(ws: &mut Workspace, view: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.toggle_popover(Popover::Branches, window, cx);
    let want_head = view == "branch-menu-current";
    let branch = ws.repo.as_ref().and_then(|r| {
        r.read(cx).branches.iter().find(|b| b.is_head == want_head && b.kind == gd_git::BranchKind::Local).cloned()
    });
    let Some(branch) = branch else { return };
    ws.open_branch_menu(branch.clone(), point(px(420.0), px(150.0)), window, cx);
    match view {
        "delete-dialog" => ws.run_branch_action(BranchAction::Delete, branch, window, cx),
        "rename-dialog" => ws.run_branch_action(BranchAction::Rename, branch, window, cx),
        _ => {}
    }
}
