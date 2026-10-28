//! The collapsible "Stashes (N)" panel in the Changes sidebar.

use gd_ui::Theme;
use gpui::{Animation, AnimationExt, App, Entity, IntoElement, RenderOnce, Window, div, ease_out_quint, prelude::*, px};

use super::stash_list::{MAX_ROWS, StashList};
use super::stash_row::STASH_ROW_HEIGHT;
use crate::components::LIST_INSET_Y;
use super::stash_panel_header::StashPanelHeader;
use crate::repo::RepoModel;
use crate::workspace::Workspace;

/// Duration of the collapse / expand slide.
const SLIDE: std::time::Duration = std::time::Duration::from_millis(200);
/// Height of [`empty_hint`].
const HINT_HEIGHT: f32 = 28.0;

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
        let (shown, collapsed, all, animate) = (visible.len(), st.collapsed, st.all_branches, st.animate);
        let (full, body) = if shown == 0 {
            (HINT_HEIGHT, empty_hint(&theme).into_any_element())
        } else {
            let list = StashList {
                repo: self.repo.clone(),
                ws: self.ws,
                visible,
                current_branch: r.current_branch().map(str::to_owned),
                all_branches: all,
            };
            ((shown.min(MAX_ROWS) as f32) * STASH_ROW_HEIGHT + LIST_INSET_Y, list.into_any_element())
        };
        let open = move |t: f32| px(full * if collapsed { 1.0 - t } else { t });
        let body = div().overflow_hidden().flex_none().child(body);
        let body = if animate {
            body.with_animation(("stash-body", collapsed as usize), Animation::new(SLIDE).with_easing(ease_out_quint()), move |el, t| el.h(open(t)))
                .into_any_element()
        } else {
            body.h(open(1.0)).into_any_element()
        };
        div()
            .flex()
            .flex_col()
            .flex_none()
            .border_t_1()
            .border_color(theme.border)
            .child(StashPanelHeader { repo: self.repo, shown, collapsed, all_branches: all })
            .child(body)
    }
}

/// Shown when the branch filter hides every stash; the header's filter
/// button switches to "All branches".
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
