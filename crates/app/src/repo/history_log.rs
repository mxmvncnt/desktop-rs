//! Paged commit log and tab switching.

use gpui::{AppContext, Context};

use super::state::{RepoModel, Tab};

/// History is paged in so huge repos open instantly.
const HISTORY_PAGE: usize = 200;

impl RepoModel {
    pub fn set_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.tab = tab;
        self.pr.open = false;
        self.force_push.open = false;
        self.pr.base_menu = false;
        if tab == Tab::History && self.log.commits.is_empty() {
            self.reload_history(cx);
        }
        cx.notify();
    }

    /// Reload the pages already loaded, keeping the selected commit if it
    /// is still there.
    pub(super) fn reload_history(&mut self, cx: &mut Context<Self>) {
        let repo = self.repo.clone();
        let count = self.log.commits.len().max(HISTORY_PAGE);
        self.log.loading = true;
        cx.spawn(async move |this, cx| {
            let res = cx.background_spawn(async move { repo.log(0, count) }).await;
            this.update(cx, |this, cx| {
                this.log.loading = false;
                match res {
                    Ok(commits) => this.apply_reloaded_log(commits, count, cx),
                    Err(e) => this.error = Some(e.to_string()),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn apply_reloaded_log(&mut self, commits: Vec<gd_git::Commit>, count: usize, cx: &mut Context<Self>) {
        let selected_sha = self.log.selected.and_then(|ix| self.log.commits.get(ix)).map(|c| c.sha.clone());
        self.log.exhausted = commits.len() < count;
        self.log.commits = commits;
        let ix = selected_sha.clone()
            .and_then(|s| self.log.commits.iter().position(|c| c.sha == s))
            .or(if self.log.commits.is_empty() { None } else { Some(0) });
        // History rewrites (amend, reword, squash, reorder, undo) can leave the
        // selection at the same index but on a different (new) sha: reload it.
        let sha_changed = ix.and_then(|i| self.log.commits.get(i)).map(|c| &c.sha) != selected_sha.as_ref();
        if ix != self.log.selected || self.log.detail.files.is_empty() || sha_changed {
            self.log.selected = None;
            if let Some(ix) = ix {
                self.select_commit(ix, cx);
            } else {
                self.log.detail.clear();
            }
        }
    }

    /// Called by the history list when the user scrolls near the end.
    pub fn load_more_history(&mut self, cx: &mut Context<Self>) {
        if self.log.loading || self.log.exhausted {
            return;
        }
        self.log.loading = true;
        let repo = self.repo.clone();
        let skip = self.log.commits.len();
        cx.spawn(async move |this, cx| {
            let res = cx.background_spawn(async move { repo.log(skip, HISTORY_PAGE) }).await;
            this.update(cx, |this, cx| {
                this.log.loading = false;
                if let Ok(mut more) = res {
                    this.log.exhausted = more.len() < HISTORY_PAGE;
                    if this.log.commits.len() == skip {
                        this.log.commits.append(&mut more);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
