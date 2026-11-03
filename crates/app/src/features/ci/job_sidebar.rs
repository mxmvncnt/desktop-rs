//! Right column of the job screen: status, timing, runner, stage and steps.

use gd_forge::Job;
use gd_ui::{Button, Icon, Theme};
use gpui::{
    App, Div, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, prelude::*, px,
};

use super::actions::CiAction;
use super::duration::{duration_text, format_duration};
use super::state::CiStore;
use super::status_badge::{look, status_badge, status_icon};
use crate::utils::text::relative_time;

pub const SIDEBAR_WIDTH: f32 = 280.0;

#[derive(IntoElement)]
pub struct JobSidebar {
    pub store: Entity<CiStore>,
    pub job: Job,
    pub actions: Vec<(CiAction, Icon, &'static str)>,
    pub acting: bool,
    pub stage: String,
}

impl RenderOnce for JobSidebar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let job = &self.job;
        let field = |label: &'static str, value: Div| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.0))
                .h(px(28.0))
                .child(div().text_color(theme.text_muted).child(label))
                .child(value)
        };
        let when = |t: Option<chrono::DateTime<chrono::Utc>>| {
            div().child(t.map_or("—".into(), |t| relative_time(t.timestamp())))
        };
        let url = job.web_url.clone();
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(SIDEBAR_WIDTH))
            .gap(px(2.0))
            .p(px(16.0))
            .border_l_1()
            .border_color(theme.border)
            .overflow_hidden()
            .when(!self.actions.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .gap(px(8.0))
                        .pb(px(10.0))
                        .children(self.actions.iter().map(|&(action, icon, label)| {
                            let store = self.store.clone();
                            Button::new(label)
                                .icon(icon)
                                .label(label)
                                .disabled(self.acting)
                                .on_click(move |_, _, cx| {
                                    store.update(cx, |s, cx| s.run_action(action, cx))
                                })
                        })),
                )
            })
            .child(field("Status", status_badge(job.status, false, theme)))
            .child(field(
                "Duration",
                div().child(duration_text(job.duration, job.started_at)),
            ))
            .child(field("Started", when(job.started_at)))
            .child(field("Finished", when(job.finished_at)))
            .child(field("Stage", div().truncate().child(self.stage.clone())))
            .child(field(
                "Runner",
                div()
                    .truncate()
                    .child(job.runner.clone().unwrap_or_else(|| "—".into())),
            ))
            .when(!job.steps.is_empty(), |d| {
                d.child(
                    div()
                        .pt(px(12.0))
                        .pb(px(4.0))
                        .text_size(px(11.0))
                        .text_color(theme.text_faint)
                        .child("Steps"),
                )
                .children(job.steps.iter().map(|step| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .h(px(26.0))
                        .child(status_icon(look(step.status, false), 14.0, theme))
                        .child(div().flex_1().truncate().child(step.name.clone()))
                        .children(step.duration.map(|s| {
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.text_faint)
                                .child(format_duration(s))
                        }))
                }))
            })
            .when_some(url, |d, url| {
                d.child(
                    div().pt(px(12.0)).child(
                        Button::new("ci-job-browser")
                            .label("Open in browser")
                            .full_width()
                            .on_click(move |_, _, cx| cx.open_url(&url)),
                    ),
                )
            })
    }
}
