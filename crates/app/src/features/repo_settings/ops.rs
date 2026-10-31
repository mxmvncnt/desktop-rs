//! `impl RepoSettingsStore`: background reads of the open repository.

use gd_git::Repository;
use gpui::Context;

use super::state::RepoSettingsStore;
use crate::repo::spawn_load;

impl RepoSettingsStore {
    /// Reset to the first page and read the repository facts off the UI thread.
    pub fn open(&mut self, repo: Repository, cx: &mut Context<Self>) {
        self.page = Default::default();
        self.load_info(repo, cx);
    }

    pub fn load_info(&mut self, repo: Repository, cx: &mut Context<Self>) {
        spawn_load(
            self,
            |this| &mut this.info_load,
            move || repo.info(),
            |this, res, cx| {
                match res {
                    Ok(info) => (this.info, this.error) = (Some(info), None),
                    Err(e) => this.error = Some(e.to_string()),
                }
                cx.notify();
            },
            cx,
        );
    }
}
