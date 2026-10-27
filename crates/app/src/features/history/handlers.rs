//! Workspace-side handlers of the history tab: open the commit menu, dialog
//! tabs and choices. Components call these through their `Entity<Workspace>` prop.

use gd_git::Commit;
use gpui::{Context, Pixels, Point, Window};

use super::commit_menu_rows::commit_menu_rows;
use super::state::{ClickKind, HistoryDialog, MenuCtx};
use crate::components::Menu;
use crate::repo::RepoModel;
use crate::workspace::{Overlay, Workspace};

impl Workspace {
    pub fn open_commit_menu(&mut self, ix: usize, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else { return };
        // Right-clicking outside the multi-selection selects that row alone.
        if !repo.read(cx).commit_selected(ix) {
            repo.update(cx, |r, cx| r.click_commit(ix, ClickKind::Plain, cx));
        }
        let Some(ctx) = menu_ctx(repo.read(cx), ix) else { return };
        let rows = commit_menu_rows(&ctx);
        let menu = Menu::new(position, rows, move |ws, action, w, cx| ws.run_hist_action(action, ctx.clone(), w, cx));
        self.open_menu(menu, window, cx);
    }
}

/// Snapshot of everything the menu rows depend on, for the commit at `ix`.
pub(crate) fn menu_ctx(r: &RepoModel, ix: usize) -> Option<MenuCtx> {
    let commit = r.commits.get(ix).cloned()?;
    let multi_ix = r.multi_indices();
    let multi = multi_ix.iter().filter_map(|&i| r.commits.get(i).cloned()).collect();
    let squashable = multi_ix.len() >= 2
        && multi_ix.windows(2).all(|w| w[1] == w[0] + 1)
        && linear_through(&r.commits, *multi_ix.last().unwrap());
    Some(MenuCtx {
        tags: r.history.tags.get(&commit.sha).cloned().unwrap_or_default(),
        unpushed: r.is_unpushed(&commit.sha),
        web_url: r.history.web_url.clone(),
        blocked: r.history_blocked(),
        is_head: ix == 0,
        can_up: ix > 0 && linear_through(&r.commits, ix),
        can_down: ix + 1 < r.commits.len() && linear_through(&r.commits, ix + 1),
        squashable,
        multi,
        commit,
    })
}

/// Commits `[..=last]` have at most one parent: the log order is then the
/// first-parent order that squash/reorder work on.
fn linear_through(commits: &[Commit], last: usize) -> bool {
    commits.iter().take(last + 1).all(|c| c.parents.len() <= 1)
}

impl Workspace {
    /// Reset dialog: the Keep-staged / Keep-unstaged tabs.
    pub(super) fn set_reset_mode(&mut self, new_mode: gd_git::ResetMode, cx: &mut Context<Self>) {
        if let Some(Overlay::History(HistoryDialog::Reset { mode, .. })) = self.overlays.last_mut() {
            *mode = new_mode;
        }
        cx.notify();
    }

    /// Cherry-pick dialog: choose the target branch row.
    pub(super) fn set_cherry_pick_branch(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(Overlay::History(HistoryDialog::CherryPick { target, .. })) = self.overlays.last_mut() {
            target.selected = ix;
        }
        cx.notify();
    }
}
