//! Virtualized list of the files of a commit, a stash or a pull request.

use gd_git::FileChange;
use gpui::{App, Entity, IntoElement, RenderOnce, UniformListScrollHandle, Window, prelude::*, px, uniform_list};

use super::FileRow;
use crate::repo::RepoModel;

#[derive(Clone, Copy)]
pub enum FileSource {
    Commit,
    Stash,
    Pr,
}

impl FileSource {
    /// Element id namespace of the rows.
    fn id(self) -> &'static str {
        match self {
            Self::Commit => "commit-file",
            Self::Stash => "stash-file",
            Self::Pr => "pr-file",
        }
    }

    /// The files and the selected index.
    fn files(self, r: &RepoModel) -> (&[FileChange], Option<usize>) {
        match self {
            Self::Commit => (&r.commit_files, r.selected_commit_file),
            Self::Stash => (&r.stashes.files, r.stashes.selected_file),
            Self::Pr => (r.pr.comparison.as_ref().map_or(&[], |c| &c.files), r.pr.selected_file),
        }
    }

    fn select(self, r: &mut RepoModel, ix: usize, cx: &mut gpui::Context<RepoModel>) {
        match self {
            Self::Commit => r.select_commit_file(ix, cx),
            Self::Stash => r.select_stash_file(ix, cx),
            Self::Pr => r.select_pr_file(ix, cx),
        }
    }
}

#[derive(IntoElement)]
pub struct FileList {
    pub repo: Entity<RepoModel>,
    pub source: FileSource,
    pub scroll: UniformListScrollHandle,
}

impl RenderOnce for FileList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (repo, source) = (self.repo, self.source);
        let count = source.files(repo.read(cx)).0.len();
        uniform_list(source.id(), count, move |range, _w, cx| {
            let (files, selected) = source.files(repo.read(cx));
            range
                .filter_map(|ix| {
                    let repo = repo.clone();
                    Some(FileRow::new(source.id(), ix, files.get(ix)?.clone(), selected == Some(ix), move |_, cx| {
                        repo.update(cx, |r, cx| source.select(r, ix, cx))
                    }))
                })
                .collect()
        })
        .track_scroll(&self.scroll)
        .px(px(6.0))
        .flex_1()
    }
}
