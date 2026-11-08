//! `impl Workspace`: opening, closing and re-syncing the pull requests page, and what its events ask of the repository.

use gd_forge::{LinePos, LineTarget};
use gd_ui::Icon;
use gpui::{Context, Pixels, Point, Window};

use super::ops::Reply;
use super::query_text::PRESETS;
use super::state::{PullTab, PullsEvent};
use crate::components::{Menu, MenuRow};
use crate::workspace::Workspace;

impl Workspace {
    /// The toolbar button and Ctrl/Cmd+Shift+R. The CI page closes: the two pages share the body.
    pub fn toggle_pulls(&mut self, cx: &mut Context<Self>) {
        if self.pulls.read(cx).open {
            self.pulls.update(cx, |p, cx| p.close(cx));
        } else {
            self.overlays.clear();
            self.ci.update(cx, |ci, cx| ci.close(cx));
            self.sync_pulls(cx);
        }
        cx.notify();
    }

    /// Open the page for the current repository, looking up its account again.
    pub fn sync_pulls(&mut self, cx: &mut Context<Self>) {
        let Some(repo) = &self.repo else { return };
        let repo = repo.read(cx).repo.clone();
        let (accounts, picks) = {
            let a = self.accounts.read(cx);
            (a.accounts.clone(), a.repo_accounts.clone())
        };
        self.pulls
            .update(cx, |p, cx| p.show(repo, accounts, picks, cx));
    }

    /// The Filters button: presets that write their query into the search field and apply it.
    pub(super) fn open_pulls_filters(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rows = PRESETS
            .iter()
            .map(|(label, query)| MenuRow::item(*query, *label, Icon::PullRequest))
            .collect();
        let menu = Menu::new(position, rows, |ws, query, _, cx| {
            ws.pulls.update(cx, |p, cx| p.set_query(query.into(), cx))
        });
        self.open_menu(menu, window, cx);
    }

    /// Esc: one screen back, then close the page. `false` when it is not open.
    pub fn back_pulls(&mut self, cx: &mut Context<Self>) -> bool {
        let pulls = self.pulls.read(cx);
        if !pulls.open {
            return false;
        }
        // Esc first drops the picked lines or the reply box.
        if pulls.pick.is_some() || pulls.replying.is_some() {
            self.pulls.update(cx, |p, cx| {
                p.pick = None;
                p.replying = None;
                cx.notify()
            });
        } else {
            self.pulls.update(cx, |p, cx| p.back(cx));
        }
        true
    }

    /// The mouse Forward button.
    pub fn forward_pulls(&mut self, cx: &mut Context<Self>) {
        if self.pulls.read(cx).open {
            self.pulls.update(cx, |p, cx| p.forward(cx));
        }
    }

    pub(crate) fn on_pulls_event(&mut self, ev: &PullsEvent, cx: &mut Context<Self>) {
        match ev {
            PullsEvent::Notice(message) => self.show_notice(message.clone(), cx),
            PullsEvent::Error(message) => self.show_error(message.clone(), cx),
            PullsEvent::Review { base, head } => {
                if let Some(repo) = &self.repo {
                    let (base, head) = (base.clone(), head.clone());
                    repo.update(cx, |r, cx| r.show_review(base, head, cx));
                }
            }
            PullsEvent::Checkout(checkout) => {
                if let Some(repo) = &self.repo {
                    let checkout = checkout.clone();
                    repo.update(cx, |r, cx| r.checkout_pull(checkout, cx));
                }
            }
        }
    }

    /// The Comment button under picked lines: a review comment on them.
    pub(super) fn post_line_comment(&mut self, cx: &mut Context<Self>) {
        let Some(repo) = &self.repo else { return };
        let pulls = self.pulls.read(cx);
        let (Some(pick), PullTab::Files) = (pulls.pick.clone(), pulls.tab) else {
            return;
        };
        let r = repo.read(cx);
        let detail = &r.review;
        let (Some(diff), Some(file)) = (
            detail.diff.as_ref(),
            detail.selected.and_then(|ix| detail.files.get(ix)),
        ) else {
            return;
        };
        if file.path != pick.path {
            return;
        }
        let pos = |ix: usize| {
            diff.lines.get(ix).map(|l| LinePos {
                old: l.old_line,
                new: l.new_line,
            })
        };
        let (Some(start), Some(end)) = (pos(pick.start), pos(pick.end)) else {
            return;
        };
        let target = LineTarget {
            path: file.path.clone(),
            old_path: file.old_path.clone().unwrap_or_else(|| file.path.clone()),
            start,
            end,
        };
        self.pulls
            .update(cx, |p, cx| p.post(Reply::Lines(target), cx));
    }
}
