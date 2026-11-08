//! Working-tree diff of the selected change. `change_detail.load` drops results
//! of older loads (the user clicked another file meanwhile).

use std::sync::Arc;

use gd_git::{Diff, DiffOptions, FileChange};
use gpui::Context;

use super::state::RepoModel;
use crate::features::diff::DiffPane;

impl RepoModel {
    pub fn select_change(&mut self, ix: usize, cx: &mut Context<Self>) {
        if self.stashes.viewing {
            // Back from the stash view to the working diff.
            self.stashes.viewing = false;
            cx.notify();
        }
        if self.work.selected == Some(ix) {
            return;
        }
        self.work.selected = Some(ix);
        self.load_change_diff(cx);
        cx.notify();
    }

    pub(super) fn load_change_diff(&mut self, cx: &mut Context<Self>) {
        let Some(change) = self.selected_change_file().cloned() else {
            self.work.detail.load.cancel();
            self.show_diff(DiffPane::Changes, None, cx);
            return;
        };
        let opts = self.diff_options;
        let path = change.path.clone();
        self.load(
            |this| &mut this.work.detail.load,
            move |repo| repo.working_diff(&change, opts),
            move |this, d, cx| this.apply_change_diff(d, path, opts, cx),
            cx,
        );
    }

    fn apply_change_diff(
        &mut self,
        d: Diff,
        path: String,
        opts: DiffOptions,
        cx: &mut Context<Self>,
    ) {
        let key = (path.clone(), opts);
        // Compare the texts, not the rows: the shown diff may be expanded.
        let unchanged = self.work.diff_key.as_ref() == Some(&key)
            && self
                .work
                .detail
                .diff
                .as_ref()
                .is_some_and(|c| match (&c.source, &d.source) {
                    (Some(a), Some(b)) => a == b,
                    _ => **c == d,
                });
        if unchanged {
            // Same file, same contents: keep the expansion and highlight.
            return;
        }
        self.work.diff_key = Some(key);
        self.show_diff(DiffPane::Changes, Some((d, path.clone())), cx);
        // A `-w` diff has other rows: `load_partial_diffs` remaps instead.
        if !opts.ignore_whitespace
            && let Some(d) = self.work.detail.diff.clone()
        {
            self.remap_selection(&path, d);
        }
    }

    /// Reload the other partially selected files and move their selections
    /// onto what is on disk now, so committing them stages what their diffs
    /// show (`stage_partial` refuses anything older).
    pub(super) fn load_partial_diffs(&mut self, cx: &mut Context<Self>) {
        let files: Vec<FileChange> = self
            .changes()
            .iter()
            .filter(|c| self.work.partial.contains_key(&c.path))
            .cloned()
            .collect();
        self.load(
            |this| &mut this.work.partial_load,
            move |repo| {
                files
                    .into_iter()
                    .map(|c| Ok((repo.working_diff(&c, DiffOptions::default())?, c.path)))
                    .collect::<gd_git::Result<Vec<_>>>()
            },
            |this, diffs, _| {
                // Without `-w` the shown file follows its own diff, so the
                // gutter and the selection always share one version.
                let shown = this
                    .selected_path()
                    .filter(|_| !this.diff_options.ignore_whitespace);
                for (d, path) in diffs {
                    if shown.as_ref() != Some(&path) {
                        this.remap_selection(&path, Arc::new(d));
                    }
                }
            },
            cx,
        );
    }

    /// Change how diffs are produced and reload the visible ones. Uses the
    /// same generation counters as selection changes, so stale results drop.
    pub fn set_diff_options(&mut self, opts: DiffOptions, cx: &mut Context<Self>) {
        if self.diff_options == opts {
            return;
        }
        self.diff_options = opts;
        self.load_change_diff(cx);
        if let Some(ix) = self.log.detail.selected {
            self.select_file(DiffPane::Commit, ix, cx);
        }
        if self.pr.open {
            self.load_pr_comparison(cx);
        }
        if self.force_push.open {
            self.load_force_push(cx);
        }
        cx.notify();
    }
}
