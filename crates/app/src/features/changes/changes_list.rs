//! Virtualized list of changed files (or the "No local changes" note).

use gd_ui::empty_state;
use gpui::{
    App, Entity, IntoElement, RenderOnce, Styled, UniformListScrollHandle, Window,
};

use super::change_row::change_row;
use crate::components::list;
use crate::repo::RepoModel;
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct ChangesList {
    pub count: usize,
    pub repo: Entity<RepoModel>,
    pub ws: Entity<Workspace>,
    pub scroll: UniformListScrollHandle,
}

impl RenderOnce for ChangesList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.count == 0 {
            return empty_state("No local changes", cx).flex_1().into_any_element();
        }
        let (repo, ws) = (self.repo, self.ws);
        list("changes", self.count, move |range, _window, cx| {
            let r = repo.read(cx);
            let rows: Vec<_> = range.filter_map(|ix| Some((ix, r.changes().get(ix)?.clone()))).collect();
            rows.into_iter().map(|(ix, change)| change_row(ix, change, &repo, &ws, cx)).collect()
        })
        .track_scroll(&self.scroll)
        .flex_1()
        .into_any_element()
    }
}
