//! Selecting a commit loads its file list and the first file's diff
//! (`DiffDetail::load` is a late slot: see `LoadSlot::late`).

use gpui::Context;

use super::state::RepoModel;
use crate::features::diff::DiffPane;

impl RepoModel {
    pub fn select_commit(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.history.multi.clear();
        if self.selected_commit == Some(ix) {
            return;
        }
        let Some(commit) = self.commits.get(ix) else {
            return;
        };
        self.selected_commit = Some(ix);
        self.load_files(DiffPane::Commit, commit.sha.clone(), cx);
        cx.notify();
    }
}
