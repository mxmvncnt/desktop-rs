//! Virtualized list of the files of a commit, a stash or a pull request.

use gpui::{App, Entity, IntoElement, RenderOnce, Window, prelude::*};

use super::{FileRow, list};
use crate::features::diff::DiffPane;
use crate::repo::RepoModel;

#[derive(IntoElement)]
pub struct FileList {
    pub repo: Entity<RepoModel>,
    /// Commit, stash or PR.
    pub pane: DiffPane,
}

impl RenderOnce for FileList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (repo, pane) = (self.repo, self.pane);
        let id = match pane {
            DiffPane::Commit => "commit-file",
            DiffPane::Stash => "stash-file",
            DiffPane::ForcePush => "force-push-file",
            _ => "pr-file",
        };
        let detail = repo.read(cx).detail(pane);
        let (count, scroll) = (detail.files.len(), detail.files_scroll.clone());
        list(id, count, move |range, _w, cx| {
            let detail = repo.read(cx).detail(pane);
            range
                .filter_map(|ix| {
                    let repo = repo.clone();
                    let file = detail.files.get(ix)?.clone();
                    Some(FileRow::new(id, ix, file, detail.selected == Some(ix), move |_, cx| {
                        repo.update(cx, |r, cx| r.select_file(pane, ix, cx))
                    }))
                })
                .collect()
        })
        .track_scroll(&scroll)
        .flex_1()
    }
}
