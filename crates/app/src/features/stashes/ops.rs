//! Stash model operations on `RepoModel`: the git work runs on the background
//! executor like the rest of `RepoModel` (see `repo/`), and stale loads are
//! dropped by the load slot of its `DiffDetail`.

use gd_git::Stash;
use gpui::{AppContext, Context};

use crate::features::diff::DiffPane;
use crate::repo::{Op, RepoModel};
use crate::utils::text::plural;

impl RepoModel {
    /// Reload the stash list in the background (called from every full refresh).
    pub fn reload_stashes(&mut self, cx: &mut Context<Self>) {
        let repo = self.repo.clone();
        cx.spawn(async move |this, cx| {
            let res = cx.background_spawn(async move { repo.stash_list() }).await;
            this.update(cx, |this, cx| {
                match res {
                    Ok(list) => {
                        if this.stashes.list != list {
                            this.stashes.list = list;
                            this.stashes_changed(cx);
                        }
                    }
                    Err(e) => log::warn!("stash list failed: {e}"),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// The list changed: drop a selection whose stash is gone.
    fn stashes_changed(&mut self, cx: &mut Context<Self>) {
        let still_there = self.stashes.selected_stash().is_some();
        if !still_there && self.stashes.selected.is_some() {
            self.stashes.selected = None;
            self.stashes.viewing = false;
            self.stashes.detail.clear();
        }
        cx.notify();
    }

    pub fn toggle_stash_panel(&mut self, cx: &mut Context<Self>) {
        self.stashes.collapsed = !self.stashes.collapsed;
        self.stashes.animate = true;
        cx.notify();
    }

    pub fn toggle_stash_filter(&mut self, cx: &mut Context<Self>) {
        self.stashes.all_branches = !self.stashes.all_branches;
        cx.notify();
    }

    /// Leave the stash diff and go back to the working changes.
    pub fn close_stash(&mut self, cx: &mut Context<Self>) {
        self.stashes.viewing = false;
        cx.notify();
    }

    pub fn select_stash(&mut self, sha: String, cx: &mut Context<Self>) {
        self.stashes.viewing = true;
        if self.stashes.selected.as_deref() == Some(&sha) && !self.stashes.detail.files.is_empty() {
            cx.notify();
            return;
        }
        self.stashes.selected = Some(sha.clone());
        self.stashes.detail.clear();
        self.load_files(DiffPane::Stash, sha, cx);
        cx.notify();
    }

    /// Stash the working changes. `paths` empty = everything.
    pub fn stash_push(&mut self, message: String, paths: Vec<String>, cx: &mut Context<Self>) {
        let n = paths.len();
        self.run(
            Op::default(),
            move |r| r.stash_push(&message, &paths),
            move |this, sha, cx| {
                this.notice = Some(if n == 0 { "Stashed all changes".into() } else { format!("Stashed {n} path{}", plural(n as u32)) });
                this.stashes.collapsed = false;
                this.stashes.selected = Some(sha);
                this.stashes.viewing = false;
                this.stashes.detail.clear();
                this.reload_stashes(cx);
            },
            cx,
        );
    }

    pub fn stash_apply(&mut self, sha: String, cx: &mut Context<Self>) {
        self.run(
            Op::default(),
            move |r| r.stash_apply(&sha),
            |this, _, _| this.notice = Some("Applied stash (kept in the list)".into()),
            cx,
        );
    }

    pub fn stash_pop(&mut self, sha: String, cx: &mut Context<Self>) {
        self.run(
            Op::default(),
            move |r| r.stash_pop(&sha),
            |this, _, cx| {
                this.notice = Some("Restored stash".into());
                this.stashes.viewing = false;
                this.reload_stashes(cx);
            },
            cx,
        );
    }

    pub fn stash_drop(&mut self, sha: String, cx: &mut Context<Self>) {
        self.run(
            Op::default(),
            move |r| r.stash_drop(&sha),
            |this, _, cx| {
                this.notice = Some("Discarded stash".into());
                this.stashes.viewing = false;
                this.reload_stashes(cx);
            },
            cx,
        );
    }

    pub fn stash_rename(&mut self, stash: Stash, message: String, cx: &mut Context<Self>) {
        self.run(
            Op::default(),
            move |r| r.stash_rename(&stash.sha, stash.branch.as_deref(), &message),
            |this, _, cx| this.reload_stashes(cx),
            cx,
        );
    }

    /// Switch branches, optionally stashing the working changes first
    /// (Desktop's "Leave my changes on <branch>"). If the checkout fails after
    /// stashing, the stash is popped again so nothing is left behind.
    pub fn checkout_with_stash(&mut self, branch: gd_git::Branch, stash: bool, cx: &mut Context<Self>) {
        if !stash {
            self.checkout_now(branch, cx);
            return;
        }
        let from = self.branch_name();
        let message = format!("Changes left on {from} when switching to {}", branch.name);
        self.run(
            Op::default(),
            move |r| {
                let sha = r.stash_push(&message, &[])?;
                if let Err(e) = r.checkout(&branch) {
                    let _ = r.stash_pop(&sha);
                    return Err(e);
                }
                Ok(())
            },
            move |this, _, cx| {
                this.reset_history_selection();
                this.last_commit = None;
                this.notice = Some(format!("Stashed your changes on {from}"));
                this.reload_stashes(cx);
            },
            cx,
        );
    }
}
