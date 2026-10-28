//! The collapsible "Stashes (N)" panel in the Changes sidebar.

use gd_ui::Theme;
use gpui::{AnyElement, App, Entity, IntoElement, RenderOnce, Window, div, prelude::*, px};

use super::stash_list::StashList;
use super::stash_panel_header::StashPanelHeader;
use crate::repo::RepoModel;
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct StashPanel {
    repo: Entity<RepoModel>,
    ws: Entity<Workspace>,
}

impl StashPanel {
    /// `None` when the repo has no stashes (the panel is hidden).
    pub fn new(repo: &Entity<RepoModel>, ws: Entity<Workspace>, cx: &App) -> Option<Self> {
        (!repo.read(cx).stashes.list.is_empty()).then(|| Self { repo: repo.clone(), ws })
    }
}

impl RenderOnce for StashPanel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let r = self.repo.read(cx);
        let st = &r.stashes;
        let visible = st.visible(r.current_branch());
        let (shown, collapsed, all) = (visible.len(), st.collapsed, st.all_branches);
        let body: Option<AnyElement> = if collapsed {
            None
        } else if shown == 0 {
            Some(empty_hint(&theme).into_any_element())
        } else {
            Some(StashList {
                repo: self.repo.clone(),
                ws: self.ws,
                visible,
                current_branch: r.current_branch().map(str::to_owned),
                all_branches: all,
            }.into_any_element())
        };
        div()
            .flex()
            .flex_col()
            .flex_none()
            .border_t_1()
            .border_color(theme.border)
            .child(StashPanelHeader { repo: self.repo, shown, collapsed, all_branches: all })
            .children(body)
    }
}

/// Shown when the branch filter hides every stash; the header's filter
/// button switches to "All branches".
fn empty_hint(theme: &Theme) -> impl IntoElement + use<> {
    div()
        .flex()
        .justify_center()
        .px(px(14.0))
        .pb(px(10.0))
        .text_size(px(12.0))
        .text_color(theme.text_faint)
        .child("No stashes on this branch")
}
