//! Snapshot views: the repository, branch, new-branch and clone popovers.

use gpui::{Context, Window};

use crate::snapshot::View;
use crate::workspace::{Popover, Workspace};

pub fn views() -> &'static [View] {
    &[
        View { name: "repos", steps: &[|ws, _, w, cx| ws.toggle_popover(Popover::Repositories, w, cx)] },
        View { name: "branches", steps: &[|ws, _, w, cx| ws.toggle_popover(Popover::Branches, w, cx)] },
        View { name: "new-branch", steps: &[|ws, _, w, cx| ws.toggle_popover(Popover::NewBranch, w, cx)] },
        View { name: "new-branch-base", steps: &[new_branch_base] },
        View { name: "clone", steps: &[|ws, _, w, cx| ws.toggle_popover(Popover::Clone, w, cx)] },
    ]
}

fn new_branch_base(ws: &mut Workspace, _: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.toggle_popover(Popover::NewBranch, window, cx);
    ws.pickers.update(cx, |p, _| {
        p.new_branch_base = Some("staging".into());
        p.new_branch_base_open = true;
    });
    ws.pickers.read(cx).name_input.clone().update(cx, |s, cx| s.emplace("feature x", cx));
}
