//! Working-tree diff of the selected change. `change_diff_load` drops results
//! of older loads (the user clicked another file meanwhile).

use std::sync::Arc;

use gd_git::{Diff, DiffOptions};
use gpui::Context;

use super::state::RepoModel;
use crate::features::diff::ExpandTarget;

impl RepoModel {
    pub fn select_change(&mut self, ix: usize, cx: &mut Context<Self>) {
        if self.stashes.viewing {
            // Back from the stash view to the working diff.
            self.stashes.viewing = false;
            cx.notify();
        }
        if self.selected_change == Some(ix) {
            return;
        }
        self.selected_change = Some(ix);
        self.load_change_diff(cx);
        cx.notify();
    }

    pub(super) fn load_change_diff(&mut self, cx: &mut Context<Self>) {
        let Some(change) = self.selected_change_file().cloned() else {
            self.change_diff_load.cancel();
            self.change_diff = None;
            self.change_hl = None;
            self._change_hl_task = None;
            return;
        };
        let opts = self.diff_options;
        let path = change.path.clone();
        self.load(
            |this| &mut this.change_diff_load,
            move |repo| repo.working_diff(&change, opts),
            move |this, d, cx| this.apply_change_diff(d, path, opts, cx),
            cx,
        );
    }

    fn apply_change_diff(&mut self, d: Diff, path: String, opts: DiffOptions, cx: &mut Context<Self>) {
        let key = (path.clone(), opts);
        let unchanged = self.change_diff_key.as_ref() == Some(&key)
            && self.change_diff.as_ref().is_some_and(|c| **c == d);
        if unchanged {
            // Same file, same contents: keep the expansion and highlight.
            return;
        }
        let d = Arc::new(d);
        self.reconcile_selection(&path, &d);
        self.change_diff_key = Some(key);
        self.change_diff = Some(d.clone());
        self.change_hl = None;
        self._change_hl_task = Some(self.spawn_highlight(d, path, ExpandTarget::Changes, cx));
    }

    /// Change how diffs are produced and reload the visible ones. Uses the
    /// same generation counters as selection changes, so stale results drop.
    pub fn set_diff_options(&mut self, opts: DiffOptions, cx: &mut Context<Self>) {
        if self.diff_options == opts {
            return;
        }
        self.diff_options = opts;
        self.load_change_diff(cx);
        if let Some(ix) = self.selected_commit_file {
            self.select_commit_file(ix, cx);
        }
        if self.pr.open {
            self.load_pr_comparison(cx);
        }
        cx.notify();
    }
}
