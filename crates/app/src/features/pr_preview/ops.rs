//! Pull-request preview operations on `RepoModel`. All git work runs on the
//! background executor; the [`LoadSlot`] drops stale loads.

use gd_git::{BranchKind, Comparison, Diff};
use gpui::Context;

use crate::features::diff::DiffPane;
use crate::repo::{RepoModel, spawn_load};

/// What one background load returns.
struct Loaded {
    base: Option<String>,
    comparison: Option<Comparison>,
    remote_url: Option<String>,
    first: Option<(Diff, String)>,
}

impl RepoModel {
    pub fn open_pr_preview(&mut self, cx: &mut Context<Self>) {
        self.pr.open = true;
        self.pr.base_menu = false;
        self.load_pr_comparison(cx);
    }

    pub fn close_pr_preview(&mut self, cx: &mut Context<Self>) {
        self.pr.open = false;
        self.pr.base_menu = false;
        self.pr.detail.load.cancel();
        cx.notify();
    }

    pub fn toggle_pr_base_menu(&mut self, cx: &mut Context<Self>) {
        self.pr.base_menu = !self.pr.base_menu;
        cx.notify();
    }

    pub fn set_pr_base(&mut self, base: String, cx: &mut Context<Self>) {
        self.pr.base = Some(base);
        self.pr.base_menu = false;
        self.load_pr_comparison(cx);
    }

    /// Branch name on the remote for the PR head: the upstream's name when
    /// the branch is published, else the local name.
    pub fn pr_head_name(&self) -> String {
        let upstream = self.work.status.as_ref().and_then(|s| s.upstream.clone());
        match upstream.and_then(|u| u.split_once('/').map(|(_, b)| b.to_string())) {
            Some(b) => b,
            None => self.branch_name(),
        }
    }

    /// `base` as the remote knows it (`origin/main` -> `main`).
    pub fn pr_base_on_remote(&self, base: &str) -> String {
        match self.branches.iter().find(|b| b.name == base) {
            Some(b) if b.kind == BranchKind::Remote => base.split_once('/').map_or(base, |(_, n)| n).to_string(),
            _ => base.to_string(),
        }
    }

    /// (Re)load the comparison of HEAD against the chosen or default base.
    pub fn load_pr_comparison(&mut self, cx: &mut Context<Self>) {
        self.pr.loading = true;
        self.pr.error = None;
        let (repo, chosen, opts) = (self.repo.clone(), self.pr.base.clone(), self.diff_options);
        spawn_load(
            self,
            |this| &mut this.pr.detail.load,
            move || {
                let base = chosen.or_else(|| repo.default_base_branch());
                let remote_url = repo.tracking_remote_url();
                let Some(b) = base.clone() else {
                    return Ok(Loaded { base, comparison: None, remote_url, first: None });
                };
                let cmp = repo.compare(&b, "HEAD")?;
                let first = match cmp.files.first() {
                    Some(f) => Some((DiffPane::Pr.file_diff(&repo, &cmp.merge_base, f, opts)?, f.path.clone())),
                    None => None,
                };
                Ok::<_, gd_git::GitError>(Loaded { base, comparison: Some(cmp), remote_url, first })
            },
            |this, res, cx| {
                this.pr.loading = false;
                match res {
                    Ok(l) => this.apply_pr_loaded(l, cx),
                    Err(e) => {
                        this.pr.comparison = None;
                        this.pr.error = Some(e.to_string());
                        this.pr.detail.clear();
                    }
                }
                cx.notify();
            },
            cx,
        );
        cx.notify();
    }

    fn apply_pr_loaded(&mut self, l: Loaded, cx: &mut Context<Self>) {
        self.pr.remote_url = l.remote_url;
        if self.pr.base.is_none() {
            self.pr.base = l.base;
        }
        match l.comparison {
            Some(c) => {
                let (merge_base, files, stats) = (c.merge_base.clone(), c.files.clone(), c.stats);
                self.pr.comparison = Some(c);
                self.show_files(DiffPane::Pr, merge_base, files, stats, l.first, cx);
            }
            None => {
                self.pr.comparison = None;
                self.pr.detail.clear();
            }
        }
    }
}
