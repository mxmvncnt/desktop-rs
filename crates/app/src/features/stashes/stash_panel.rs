//! The collapsible "Stashes (N)" panel in the Changes sidebar.

use gd_ui::Theme;
use gpui::{
    Animation, AnimationExt, App, Entity, IntoElement, RenderOnce, Window, div, ease_out_quint,
    prelude::*, px,
};

use super::stash_list::{StashList, list_height};
use super::stash_panel_header::StashPanelHeader;
use super::state::StashState;
use crate::repo::RepoModel;
use crate::workspace::Workspace;

/// Duration of the slide when the body collapses, expands or changes height.
const SLIDE: std::time::Duration = std::time::Duration::from_millis(200);
/// Height of [`empty_hint`].
const HINT_HEIGHT: f32 = 28.0;

/// Height of the panel body: nothing when collapsed, else the list or the empty hint.
pub fn body_height(st: &StashState, current_branch: Option<&str>) -> f32 {
    match (st.expanded, st.visible(current_branch).len()) {
        (false, _) => 0.0,
        (true, 0) => HINT_HEIGHT,
        (true, n) => list_height(n),
    }
}

#[derive(IntoElement)]
pub struct StashPanel {
    repo: Entity<RepoModel>,
    ws: Entity<Workspace>,
}

impl StashPanel {
    /// `None` when the repo has no stashes (the panel is hidden).
    pub fn new(repo: &Entity<RepoModel>, ws: Entity<Workspace>, cx: &App) -> Option<Self> {
        (!repo.read(cx).stashes.list.is_empty()).then(|| Self {
            repo: repo.clone(),
            ws,
        })
    }
}

impl RenderOnce for StashPanel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let r = self.repo.read(cx);
        let st = &r.stashes;
        let visible = st.visible(r.current_branch());
        let (shown, collapsed, all) = (visible.len(), !st.expanded, st.all_branches);
        let target = body_height(st, r.current_branch());
        let body = if shown == 0 {
            empty_hint(&theme).into_any_element()
        } else {
            StashList {
                repo: self.repo.clone(),
                ws: self.ws,
                visible,
                current_branch: r.current_branch().map(str::to_owned),
            }
            .into_any_element()
        };
        let body = div().overflow_hidden().flex_none().child(body);
        let body = match st.slide_from {
            Some(from) => body
                .with_animation(
                    ("stash-body", st.slides),
                    Animation::new(SLIDE).with_easing(ease_out_quint()),
                    move |el, t| el.h(px(from + (target - from) * t)),
                )
                .into_any_element(),
            None => body.h(px(target)).into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .flex_none()
            .border_t_1()
            .border_color(theme.border)
            .child(StashPanelHeader {
                repo: self.repo,
                shown,
                collapsed,
                all_branches: all,
            })
            .child(body)
    }
}

/// Shown when the branch filter hides every stash; the header's
/// "Current branch only" checkbox shows all of them.
fn empty_hint(theme: &Theme) -> impl IntoElement + use<> {
    div()
        .flex()
        .justify_center()
        .px(px(14.0))
        .h(px(HINT_HEIGHT))
        .text_size(px(12.0))
        .text_color(theme.text_faint)
        .child("No stashes on this branch")
}
