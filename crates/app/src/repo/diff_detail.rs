//! The file list + selected file + diff + syntax colors that the changes,
//! history, stash and pull-request panes each show ([`DiffDetail`]), and the
//! one set of operations that drives them.

use std::sync::Arc;

use gd_git::{Diff, DiffOptions, Expansion, FileChange, Repository};
use gpui::{Context, UniformListScrollHandle};

use super::run::{LoadSlot, spawn_load};
use super::state::RepoModel;
use crate::features::diff::{DiffHighlight, DiffPane, highlight_diff};

#[derive(Default)]
pub struct DiffDetail {
    /// Commit / stash files or PR comparison files (unused by the changes pane,
    /// whose list is the working status).
    pub files: Vec<FileChange>,
    pub selected: Option<usize>,
    /// The commit or stash sha, or PR merge base, that `files` belong to. Lags
    /// behind the selection while a load is in flight, so a pane switches in one go.
    pub base: Option<String>,
    pub diff: Option<Arc<Diff>>,
    /// Path of the file `diff` was loaded for.
    pub path: Option<String>,
    /// Syntax colors for `diff`; `None` until the background pass lands.
    pub hl: Option<Arc<DiffHighlight>>,
    pub files_scroll: UniformListScrollHandle,
    pub(crate) load: LoadSlot,
    pub(super) hl_load: LoadSlot,
}

impl DiffDetail {
    /// Forget everything and drop the loads in flight.
    pub fn clear(&mut self) {
        self.files.clear();
        self.selected = None;
        self.base = None;
        self.diff = None;
        self.path = None;
        self.hl = None;
        self.load.cancel();
        self.hl_load.cancel();
    }
}

impl DiffPane {
    /// Diff of `file` within `base` (commit or stash sha, PR merge base).
    pub(crate) fn file_diff(self, repo: &Repository, base: &str, file: &FileChange, opts: DiffOptions) -> gd_git::Result<Diff> {
        match self {
            Self::Changes => repo.working_diff(file, opts),
            Self::Commit => repo.commit_diff(base, file, opts),
            Self::Stash => repo.stash_diff(base, file),
            Self::Pr => repo.compare_file_diff(base, "HEAD", file, opts),
        }
    }
}

impl RepoModel {
    pub fn detail(&self, pane: DiffPane) -> &DiffDetail {
        match pane {
            DiffPane::Changes => &self.change_detail,
            DiffPane::Commit => &self.commit_detail,
            DiffPane::Stash => &self.stashes.detail,
            DiffPane::Pr => &self.pr.detail,
        }
    }

    pub(crate) fn detail_mut(&mut self, pane: DiffPane) -> &mut DiffDetail {
        match pane {
            DiffPane::Changes => &mut self.change_detail,
            DiffPane::Commit => &mut self.commit_detail,
            DiffPane::Stash => &mut self.stashes.detail,
            DiffPane::Pr => &mut self.pr.detail,
        }
    }

    /// Show `diff` (with the path it was loaded for) and highlight it in the
    /// background. A newer diff drops the highlight still in flight.
    pub(crate) fn show_diff(&mut self, pane: DiffPane, diff: Option<(Diff, String)>, cx: &mut Context<Self>) {
        let cache = self.hl_cache.clone();
        let detail = self.detail_mut(pane);
        detail.hl = None;
        let Some((diff, path)) = diff else {
            detail.diff = None;
            detail.path = None;
            detail.hl_load.cancel();
            return;
        };
        let diff = Arc::new(diff);
        detail.diff = Some(diff.clone());
        detail.path = Some(path.clone());
        spawn_load(
            self,
            move |this| &mut this.detail_mut(pane).hl_load,
            move || highlight_diff(&diff, &path, &cache),
            move |this, hl, cx| {
                if hl.is_some() {
                    this.detail_mut(pane).hl = hl;
                    cx.notify();
                }
            },
            cx,
        );
    }

    /// Replace the file list of `base` and show the diff of its first file.
    pub(crate) fn show_files(
        &mut self,
        pane: DiffPane,
        base: String,
        files: Vec<FileChange>,
        first: Option<(Diff, String)>,
        cx: &mut Context<Self>,
    ) {
        let detail = self.detail_mut(pane);
        detail.selected = (!files.is_empty()).then_some(0);
        detail.files = files;
        detail.base = Some(base);
        self.show_diff(pane, first, cx);
    }

    /// List the files of commit or stash `sha` and show the first one's diff.
    pub(crate) fn load_files(&mut self, pane: DiffPane, sha: String, cx: &mut Context<Self>) {
        let opts = self.diff_options;
        let loaded = sha.clone();
        self.load(
            move |this| &mut this.detail_mut(pane).load,
            move |repo| {
                let files = match pane {
                    DiffPane::Stash => repo.stash_files(&sha)?,
                    _ => repo.commit_files(&sha)?,
                };
                let first = match files.first() {
                    Some(f) => Some((pane.file_diff(repo, &sha, f, opts)?, f.path.clone())),
                    None => None,
                };
                Ok((files, first))
            },
            move |this, (files, first), cx| this.show_files(pane, loaded, files, first, cx),
            cx,
        );
    }

    /// Show the diff of file `ix` of the commit, stash or comparison.
    pub fn select_file(&mut self, pane: DiffPane, ix: usize, cx: &mut Context<Self>) {
        let detail = self.detail_mut(pane);
        let (Some(base), Some(file)) = (detail.base.clone(), detail.files.get(ix).cloned()) else {
            return;
        };
        detail.selected = Some(ix);
        let opts = self.diff_options;
        self.load(
            move |this| &mut this.detail_mut(pane).load,
            move |repo| Ok((pane.file_diff(repo, &base, &file, opts)?, file.path)),
            move |this, d, cx| this.show_diff(pane, Some(d), cx),
            cx,
        );
        cx.notify();
    }

    /// Reveal more unchanged lines around a hunk of the displayed diff. Pure
    /// and cheap (the diff holds both texts), so it runs inline; highlighting
    /// is redone in the background.
    pub fn expand_diff(&mut self, pane: DiffPane, e: Expansion, cx: &mut Context<Self>) {
        let detail = self.detail(pane);
        let (Some(diff), Some(path)) = (&detail.diff, detail.path.clone()) else { return };
        let Some(expanded) = gd_git::expand(diff, e) else { return };
        self.show_diff(pane, Some((expanded, path)), cx);
        cx.notify();
    }
}
