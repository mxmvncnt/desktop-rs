//! Snapshot views: history tab, commit menus and dialogs. Need a repo with a remote (see PROGRESS.md).

use gpui::{Context, Window, point, px};

use super::handlers::menu_ctx;
use super::state::{ClickKind, HistoryAction};
use crate::features::diff::DiffPane;
use crate::repo::Tab;
use crate::snapshot::{Step, View, press_next};
use crate::workspace::{NavList, Workspace};

fn history_tab(ws: &mut Workspace, _: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    if let Some(r) = &ws.repo {
        r.update(cx, |r, cx| r.set_tab(Tab::History, cx));
    }
}

const COMMITS: Step = |ws, _, w, cx| press_next(Some(NavList::Commits), ws, w, cx);

pub fn views() -> &'static [View] {
    &[
        View { name: "history", steps: &[history_tab] },
        View { name: "history-*", steps: &[history_tab, commit_state] },
        View { name: "amend-mode", steps: &[history_tab, commit_state] },
        View {
            name: "nav-history",
            steps: &[
                history_tab,
                COMMITS,
                COMMITS,
                |ws, _, w, cx| press_next(Some(NavList::Files(DiffPane::Commit)), ws, w, cx),
            ],
        },
    ]
}

/// Needs the history list loaded: `history-menu`, `-menu-head`, `-menu-pushed`, `-multiselect`,
/// `-multimenu`, `-squash`, `-reset`, `-cherry`, `-tag`, `-checkout`, `-amend`, `-msg-tall`, `-search`, `amend-mode`.
fn commit_state(ws: &mut Workspace, view: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    let Some(repo) = ws.repo.clone() else { return };
    let at = point(px(150.0), px(190.0));
    // Open the menu of commit 2 and pick `action` as a click would.
    let pick = |ws: &mut Workspace, action, window: &mut Window, cx: &mut Context<Workspace>| {
        ws.open_commit_menu(2, at, window, cx);
        if let Some(ctx) = menu_ctx(repo.read(cx), 2) {
            ws.run_hist_action(action, ctx, window, cx);
        }
    };
    let multi = |cx: &mut Context<Workspace>| {
        repo.update(cx, |r, cx| {
            r.click_commit(1, ClickKind::Plain, cx);
            r.click_commit(3, ClickKind::Range, cx);
        })
    };
    match view {
        // Amend HEAD with the (included) working changes while HEAD is selected.
        "history-amend" => repo.update(cx, |r, cx| r.amend("Amended summary".into(), "Amended body".into(), cx)),
        "history-search" => ws.history_ui.search.update(cx, |s, cx| s.emplace("module 3", cx)),
        "history-msg-tall" => {
            ws.commit_msg.height = Some(330.0);
            cx.notify();
        }
        "history-menu" => ws.open_commit_menu(2, at, window, cx),
        "history-menu-head" => ws.open_commit_menu(0, at, window, cx),
        "history-menu-pushed" => ws.open_commit_menu(6, at, window, cx),
        "history-multiselect" => multi(cx),
        "history-multimenu" => {
            multi(cx);
            ws.open_commit_menu(2, at, window, cx);
        }
        "history-squash" => {
            multi(cx);
            pick(ws, HistoryAction::Squash, window, cx);
        }
        "history-reset" => pick(ws, HistoryAction::Reset, window, cx),
        "history-cherry" => pick(ws, HistoryAction::CherryPick, window, cx),
        "history-tag" => {
            pick(ws, HistoryAction::CreateTag, window, cx);
            ws.history_ui.name.update(cx, |s, cx| s.emplace("v1.2.0", cx));
        }
        "history-checkout" => pick(ws, HistoryAction::Checkout, window, cx),
        "amend-mode" => {
            let head = repo.read(cx).log.commits.first().cloned();
            if let Some(head) = head {
                ws.start_amend(head, window, cx);
            }
        }
        _ => {}
    }
    cx.notify();
}
