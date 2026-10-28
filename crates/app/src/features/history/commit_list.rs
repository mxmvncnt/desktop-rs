//! Virtualized, infinitely-paged list of commits (History tab sidebar).

use gd_ui::empty_state;
use gpui::{App, Entity, IntoElement, RenderOnce, UniformListScrollHandle, Window, div, prelude::*};

use gpui_elements::editable_text::EditableTextState;

use crate::components::list;
use super::commit_row::CommitRow;
use super::search::matching_commits;
use super::search_box::SearchBox;
use crate::repo::RepoModel;
use crate::workspace::{NavList, Workspace, nav_focus};

/// Load the next page when the visible range gets this close to the end.
const PAGE_AHEAD: usize = 30;
/// While searching, keep paging in until this many rows match (or history ends).
const MIN_MATCHES: usize = 50;

#[derive(IntoElement)]
pub struct CommitList {
    pub repo: Entity<RepoModel>,
    pub ws: Entity<Workspace>,
    pub scroll: UniformListScrollHandle,
    pub search: Entity<EditableTextState>,
}

impl RenderOnce for CommitList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let query = self.search.read(cx).as_str().to_string();
        let body = self.body(&query, cx);
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(SearchBox { input: self.search })
            .child(body)
    }
}

impl CommitList {
    fn body(&self, query: &str, cx: &mut App) -> gpui::AnyElement {
        let (repo, ws) = (self.repo.clone(), self.ws.clone());
        let (total, visible) = {
            let r = repo.read(cx);
            (r.log.commits.len(), matching_commits(&r.log.commits, query))
        };
        let searching = visible.is_some();
        let n = visible.as_ref().map_or(total, Vec::len);
        if searching {
            // Matches only come from loaded pages: fetch more while there are few.
            if n < MIN_MATCHES {
                repo.update(cx, |r, cx| r.load_more_history(cx));
            }
        }
        if n == 0 {
            return empty_state(if searching { "No matching commits" } else { "No commits yet" }, cx)
                .flex_1()
                .into_any_element();
        }
        let nav_ws = ws.clone();
        let list = list("history", n, move |range, _w, cx| {
            // Infinite scroll: page in more commits near the end.
            if range.end + PAGE_AHEAD >= n {
                repo.update(cx, |r, cx| r.load_more_history(cx));
            }
            range
                .filter_map(|row| {
                    let ix = visible.as_ref().map_or(row, |v| v[row]);
                    CommitRow::from_model(ix, &repo, &ws, cx)
                })
                .collect()
        })
        .track_scroll(&self.scroll)
        .flex_1();
        nav_focus(div(), NavList::Commits, nav_ws).flex().flex_col().flex_1().min_h_0().child(list).into_any_element()
    }
}
