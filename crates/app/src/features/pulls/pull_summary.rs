//! Head of an open pull request: state, title, the branches, and Check out / Open in browser / Refresh.

use gd_forge::PullRequest;
use gd_ui::{Button, Icon, Theme, badge, icon};
use gpui::{
    App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, prelude::*, px,
};

use super::state::PullsStore;
use super::state_badge::state_badge;
use crate::utils::text::relative_time;

#[derive(IntoElement)]
pub struct PullSummary {
    pub store: Entity<PullsStore>,
    pub pull: PullRequest,
}

impl RenderOnce for PullSummary {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let pulls = self.store.read(cx);
        let p = &self.pull;
        let fetch_error = match &pulls.head {
            Some(Err(e)) => Some(e.clone()),
            _ => None,
        };
        let can_checkout = pulls.pull_head().is_some();
        let (checkout, refresh) = (self.store.clone(), self.store.clone());
        let url = p.web_url.clone();
        let author = p
            .author
            .as_ref()
            .map(|u| u.username.clone())
            .unwrap_or_default();
        let created = p
            .created_at
            .map(|t| relative_time(t.timestamp()))
            .unwrap_or_default();
        let branch = |name: Option<&String>| {
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .px(px(6.0))
                .rounded(px(4.0))
                .bg(theme.element_hover)
                .font_family(theme.font_mono.clone())
                .text_size(px(11.5))
                .text_color(theme.text)
                .child(
                    icon(Icon::GitBranch)
                        .size(px(11.0))
                        .text_color(theme.text_faint),
                )
                .child(name.cloned().unwrap_or_else(|| "?".into()))
        };
        div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(10.0))
            .px(px(24.0))
            .pb(px(14.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(state_badge(p, theme))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(16.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(format!("{} #{}", p.title, p.number)),
                    )
                    .child(
                        Button::new("pull-checkout")
                            .icon(Icon::GitBranch)
                            .label("Check out")
                            .disabled(!can_checkout)
                            .on_click(move |_, _, cx| checkout.update(cx, |s, cx| s.checkout(cx))),
                    )
                    .child(
                        Button::new("pull-open-browser")
                            .icon(Icon::Globe)
                            .label("Open in browser")
                            .on_click(move |_, _, cx| cx.open_url(&url)),
                    )
                    .child(
                        Button::new("pull-refresh")
                            .icon(Icon::Refresh)
                            .label("Refresh")
                            .on_click(move |_, _, cx| {
                                refresh.update(cx, |s, cx| s.refresh_pull(cx))
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .text_size(px(12.0))
                    .text_color(theme.text_muted)
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child(author),
                    )
                    .child("wants to merge")
                    .child(branch(p.head_ref.as_ref()))
                    .child("into")
                    .child(branch(p.base_ref.as_ref()))
                    .child(format!("· opened {created}"))
                    .when(p.from_fork, |d| d.child(badge("fork", cx)))
                    .children(p.labels.iter().map(|l| badge(l.clone(), cx))),
            )
            .children(fetch_error.map(|e| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .text_size(px(12.0))
                    .text_color(theme.warning)
                    .child(icon(Icon::Warning).size(px(13.0)))
                    .child(format!("Couldn't fetch the pull request's commits: {e}"))
            }))
    }
}
