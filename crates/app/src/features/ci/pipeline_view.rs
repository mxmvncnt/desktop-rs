//! A pipeline: the breadcrumbs, summary, and the graph / jobs tabs.

use gd_forge::{Forge, Status};
use gd_ui::{Button, Icon, Tabs, Theme, divider, empty_state, icon};
use gpui::{
    App, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};

use super::actions::CiAction;
use super::breadcrumbs::breadcrumbs;
use super::duration::duration_text;
use super::jobs_table::JobsTable;
use super::stage_column::stage_column;
use super::state::{CiStore, PipelineTab};
use super::status_badge::status_badge;
use crate::components::ProblemView;
use crate::features::accounts::Subject;
use crate::utils::text::relative_time;
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct PipelineView {
    pub store: Entity<CiStore>,
    pub ws: Entity<Workspace>,
    pub id: gd_forge::PipelineId,
}

impl RenderOnce for PipelineView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let ci = self.store.read(cx);
        let Some(detail) = ci.details.get(&self.id).cloned() else {
            return match &ci.detail_error {
                Some(problem) => {
                    let ws = self.ws.clone();
                    ProblemView::new(
                        problem.clone(),
                        Subject::Pipelines,
                        ci.refreshing,
                        self.ws,
                        move |_, cx| ws.update(cx, |ws, cx| ws.sync_ci(cx)),
                    )
                    .into_any_element()
                }
                None => empty_state("Loading pipeline…", cx)
                    .flex_1()
                    .into_any_element(),
            };
        };
        let (tab, store, acting) = (ci.tab, self.store.clone(), ci.acting);
        let caps = ci
            .ready()
            .map(|t| t.forge.capabilities())
            .unwrap_or_else(|| Forge::GitLab.capabilities());
        let p = &detail.pipeline;
        let jobs = detail.stages.iter().map(|s| s.jobs.len()).sum::<usize>();
        let meta = |text: String| div().text_color(theme.text_muted).child(text);
        let created = p
            .created_at
            .map(|t| relative_time(t.timestamp()))
            .unwrap_or_default();
        let user = p
            .user
            .as_ref()
            .map(|u| u.name.clone().unwrap_or_else(|| u.username.clone()));
        let url = p.web_url.clone();
        let id = p.id;
        let sha: String = p.sha.chars().take(8).collect();

        let summary = div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(status_badge(p.status, detail.passed_with_warnings(), theme))
            .child(
                div()
                    .flex_1()
                    .truncate()
                    .text_size(px(16.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(
                        p.title
                            .clone()
                            .or_else(|| p.workflow.clone())
                            .unwrap_or_else(|| format!("Pipeline #{}", p.number)),
                    ),
            )
            .when(
                caps.retry_pipeline && matches!(p.status, Status::Failed | Status::Canceled),
                |d| {
                    let store = store.clone();
                    d.child(
                        Button::new("ci-retry-pipeline")
                            .icon(Icon::Restart)
                            .label("Retry failed")
                            .disabled(acting)
                            .on_click(move |_, _, cx| {
                                store.update(cx, |s, cx| {
                                    s.run_action(CiAction::RetryPipeline(id), cx)
                                })
                            }),
                    )
                },
            )
            .when(caps.cancel_pipeline && !p.status.is_finished(), |d| {
                let store = store.clone();
                d.child(
                    Button::new("ci-cancel-pipeline")
                        .icon(Icon::Stop)
                        .label("Cancel")
                        .disabled(acting)
                        .on_click(move |_, _, cx| {
                            store.update(cx, |s, cx| s.run_action(CiAction::CancelPipeline(id), cx))
                        }),
                )
            })
            .child(
                Button::new("ci-open-browser")
                    .label("Open in browser")
                    .on_click(move |_, _, cx| cx.open_url(&url)),
            );

        let facts = div()
            .flex()
            .items_center()
            .gap(px(16.0))
            .text_size(px(12.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .text_color(theme.text_muted)
                    .child(
                        icon(Icon::GitBranch)
                            .size(px(12.0))
                            .text_color(theme.text_faint),
                    )
                    .child(p.git_ref.clone()),
            )
            .child(
                div()
                    .font_family(theme.font_mono.clone())
                    .text_color(theme.text_faint)
                    .child(sha),
            )
            .children(user.map(meta))
            .child(meta(duration_text(p.duration, p.started_at)))
            .child(meta(format!("created {created}")));

        let body = match tab {
            PipelineTab::Graph => div()
                .id("ci-graph")
                .flex_1()
                .min_h_0()
                .overflow_scroll()
                .child(
                    div().flex().items_start().p(px(24.0)).children(
                        detail
                            .stages
                            .iter()
                            .enumerate()
                            .flat_map(|(ix, stage)| {
                                // A line between two columns, level with the stage headers.
                                let link = (ix > 0).then(|| {
                                    div()
                                        .flex_none()
                                        .w(px(32.0))
                                        .h(px(36.0))
                                        .flex()
                                        .items_center()
                                        .child(div().h(px(1.0)).w_full().bg(theme.border_strong))
                                });
                                [
                                    link.map(IntoElement::into_any_element),
                                    Some(stage_column(stage, caps, &store, cx).into_any_element()),
                                ]
                            })
                            .flatten(),
                    ),
                )
                .into_any_element(),
            PipelineTab::Jobs => div()
                .id("ci-jobs")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(px(14.0))
                .py(px(12.0))
                .child(JobsTable {
                    store: store.clone(),
                    detail: detail.clone(),
                })
                .into_any_element(),
        };

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(breadcrumbs(&store, p.number, None))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_none()
                    .gap(px(12.0))
                    .px(px(24.0))
                    .pb(px(14.0))
                    .child(summary)
                    .child(facts),
            )
            .child(divider(cx))
            .child(
                div()
                    .flex_none()
                    .w(px(260.0))
                    .mx(px(24.0))
                    .my(px(12.0))
                    .child(
                        Tabs::new("ci-pipeline-tabs", tab as usize)
                            .tab("Pipeline", None)
                            .tab("Jobs", Some(jobs.to_string().into()))
                            .on_select(move |ix, _, cx| {
                                store.update(cx, |s, cx| {
                                    s.tab = if ix == 0 {
                                        PipelineTab::Graph
                                    } else {
                                        PipelineTab::Jobs
                                    };
                                    cx.notify()
                                })
                            }),
                    ),
            )
            .child(body)
            .into_any_element()
    }
}
