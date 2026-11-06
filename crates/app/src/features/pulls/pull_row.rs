//! One row of the pull requests list (a plain fn: virtualized rows are rebuilt every frame).

use gd_forge::PullRequest;
use gd_ui::{Icon, Theme, badge, icon, pressable};
use gpui::{
    App, Entity, IntoElement, ParentElement, SharedString, StatefulInteractiveElement, Styled, div,
    px,
};

use super::state::PullsStore;
use super::state_badge::state_badge;
use crate::utils::text::relative_time;

pub const ROW_HEIGHT: f32 = 60.0;

pub fn pull_row(
    ix: usize,
    p: &PullRequest,
    store: Entity<PullsStore>,
    cx: &App,
) -> impl IntoElement + use<> {
    let theme = Theme::of(cx);
    let number = p.number;
    let author = p
        .author
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_default();
    let updated = p
        .updated_at
        .map(|t| relative_time(t.timestamp()))
        .unwrap_or_default();
    let muted = |text: String| div().flex_none().text_color(theme.text_muted).child(text);
    pressable(("pull-row", ix), false, cx)
        .flex()
        .items_center()
        .gap(px(12.0))
        .h(px(ROW_HEIGHT))
        .w_full()
        .px(px(10.0))
        .rounded(px(6.0))
        .on_click(move |_, _, cx| store.update(cx, |s, cx| s.open_pull(number, cx)))
        .child(div().w(px(84.0)).flex_none().child(state_badge(p, theme)))
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(px(3.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .min_w_0()
                        .child(
                            div()
                                .truncate()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child(SharedString::from(p.title.clone())),
                        )
                        .children(p.labels.iter().take(3).map(|l| badge(l.clone(), cx))),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .text_size(px(12.0))
                        .child(muted(format!("#{number}")))
                        .child(muted(format!("by {author}")))
                        .children(p.head_ref.clone().map(|h| {
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .min_w_0()
                                .text_color(theme.text_faint)
                                .child(
                                    icon(Icon::GitBranch)
                                        .size(px(11.0))
                                        .text_color(theme.text_faint),
                                )
                                .child(div().truncate().child(h))
                        })),
                ),
        )
        .children(p.comments.filter(|n| *n > 0).map(|n| {
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap(px(4.0))
                .text_size(px(12.0))
                .text_color(theme.text_muted)
                .child(icon(Icon::Pen).size(px(12.0)).text_color(theme.text_muted))
                .child(n.to_string())
        }))
        .child(
            div()
                .w(px(110.0))
                .flex_none()
                .text_size(px(12.0))
                .text_color(theme.text_faint)
                .child(updated),
        )
}
