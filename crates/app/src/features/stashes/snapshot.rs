//! Snapshot views: stash list, detail, menu and dialogs (use a repo with stashes).

use gpui::{Context, Window, point, px};

use super::StashDialog;
use crate::features::diff::DiffPane;
use crate::snapshot::{View, press_next};
use crate::workspace::{NavList, Overlay, Workspace};

pub fn views() -> &'static [View] {
    &[
        View { name: "stash-detail", steps: &[|ws, _, _, cx| select_stash(ws, true, cx)] },
        View { name: "stash-detail-untracked", steps: &[|ws, _, _, cx| select_stash(ws, false, cx)] },
        View { name: "stash-all", steps: &[stash_all] },
        View { name: "stash-menu", steps: &[stash_menu] },
        View { name: "stash-dialog", steps: &[stash_dialog] },
        View { name: "stash-drop-dialog", steps: &[stash_drop_dialog] },
        View { name: "switch-branch-dialog", steps: &[switch_branch_dialog] },
        View {
            name: "nav-stash",
            steps: &[|ws, _, _, cx| select_stash(ws, true, cx), |ws, _, w, cx| press_next(Some(NavList::Files(DiffPane::Stash)), ws, w, cx)],
        },
    ]
}

/// Select the first (or last) stash of all branches.
fn select_stash(ws: &mut Workspace, first: bool, cx: &mut Context<Workspace>) {
    let Some(r) = &ws.repo else { return };
    r.update(cx, |r, cx| {
        r.stashes.all_branches = true;
        let stash = if first { r.stashes.list.first() } else { r.stashes.list.last() };
        if let Some(s) = stash.cloned() {
            r.select_stash(s.sha, cx);
        }
    });
}

fn stash_all(ws: &mut Workspace, _: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    if let Some(r) = &ws.repo {
        r.update(cx, |r, cx| {
            r.stashes.all_branches = true;
            cx.notify()
        });
    }
}

fn stash_menu(ws: &mut Workspace, _: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    if let Some(s) = ws.repo.as_ref().and_then(|r| r.read(cx).stashes.list.first().cloned()) {
        ws.open_stash_menu(s, point(px(120.0), px(560.0)), window, cx);
    }
}

fn stash_dialog(ws: &mut Workspace, _: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    if let Some(r) = &ws.repo {
        r.update(cx, |r, _| r.work.excluded.insert("untracked.md".into()));
    }
    ws.open_stash_create(false, window, cx);
}

fn stash_drop_dialog(ws: &mut Workspace, _: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    if let Some(s) = ws.repo.as_ref().and_then(|r| r.read(cx).stashes.list.first().cloned()) {
        ws.open_overlay(Overlay::Stash(StashDialog::Drop(s)), cx);
    }
}

fn switch_branch_dialog(ws: &mut Workspace, _: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    if let Some(b) = ws.repo.as_ref().and_then(|r| r.read(cx).branches.iter().find(|b| !b.is_head).cloned()) {
        ws.open_switch_dialog(b, cx);
    }
}
