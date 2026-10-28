//! The virtualized list of stash rows inside the sidebar panel.

use gpui::{App, Entity, IntoElement, RenderOnce, Window, div, prelude::*, px};

use crate::components::{LIST_INSET_Y, list};
use super::stash_row::{STASH_ROW_HEIGHT, StashRow};
use crate::repo::RepoModel;
use crate::workspace::{NavList, Workspace, nav_focus};

/// Rows visible before the panel's list scrolls.
pub const MAX_ROWS: usize = 4;

#[derive(IntoElement)]
pub struct StashList {
    pub repo: Entity<RepoModel>,
    pub ws: Entity<Workspace>,
    /// Indices into `StashState::list` that pass the branch filter.
    pub visible: Vec<usize>,
    pub current_branch: Option<String>,
    pub all_branches: bool,
}

impl RenderOnce for StashList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let n = self.visible.len();
        let st = &self.repo.read(cx).stashes;
        let (scroll, viewing, selected) = (st.list_scroll.clone(), st.viewing, st.selected.clone());
        let nav_ws = self.ws.clone();
        let Self { repo, ws, visible, current_branch, all_branches } = self;
        let list = list("stashes", n, move |range, _w, cx| {
            let r = repo.read(cx);
            range
                .filter_map(|i| {
                    let s = r.stashes.list.get(*visible.get(i)?)?.clone();
                    let here = s.branch.is_some() && s.branch == current_branch;
                    let selected_here = viewing && selected.as_deref() == Some(s.sha.as_str());
                    Some(StashRow {
                        stash: s,
                        selected: selected_here,
                        here,
                        all_branches,
                        repo: repo.clone(),
                        ws: ws.clone(),
                    })
                })
                .collect()
        })
        .track_scroll(&scroll)
        .size_full();
        nav_focus(div(), NavList::Stashes, nav_ws)
            .h(px(n.min(MAX_ROWS) as f32 * STASH_ROW_HEIGHT + LIST_INSET_Y))
            .flex_none()
            .child(list)
    }
}
