//! The "Jobs" tab of a pipeline: one row per job.

use gd_forge::PipelineDetail;
use gd_ui::{Theme, pressable};
use gpui::{App, Div, Entity, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled, Window, div, px};

use super::duration::duration_text;
use super::state::CiStore;
use super::status_badge::status_badge;
use crate::utils::text::relative_time;

const STATUS_W: f32 = 112.0;
const STAGE_W: f32 = 160.0;
const RUNNER_W: f32 = 170.0;
const DURATION_W: f32 = 100.0;
const FINISHED_W: f32 = 130.0;

#[derive(IntoElement)]
pub struct JobsTable {
    pub store: Entity<CiStore>,
    pub detail: PipelineDetail,
}

fn cell(width: Option<f32>) -> Div {
    let cell = div().flex().items_center().min_w_0().overflow_hidden();
    match width {
        Some(w) => cell.w(px(w)).flex_none(),
        None => cell.flex_1(),
    }
}

impl RenderOnce for JobsTable {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let row = || div().flex().items_center().gap(px(12.0)).px(px(10.0));
        let header = row()
            .h(px(30.0))
            .border_b_1()
            .border_color(theme.border)
            .text_size(px(11.0))
            .text_color(theme.text_faint)
            .child(cell(Some(STATUS_W)).child("Status"))
            .child(cell(None).child("Job"))
            .child(cell(Some(STAGE_W)).child("Stage"))
            .child(cell(Some(RUNNER_W)).child("Runner"))
            .child(cell(Some(DURATION_W)).child("Duration"))
            .child(cell(Some(FINISHED_W)).child("Finished"));
        let jobs = self.detail.stages.iter().flat_map(|stage| stage.jobs.iter().map(move |job| (stage, job)));
        div().flex().flex_col().child(header).children(jobs.map(|(stage, job)| {
            let finished = job.finished_at.map_or("—".into(), |t| relative_time(t.timestamp()));
            let (store, id) = (self.store.clone(), job.id);
            pressable(("jobs-row", job.id.id as usize), false, cx)
                .rounded(px(6.0))
                .on_click(move |_, _, cx| store.update(cx, |s, cx| s.open_job(id, cx)))
                .child(
                    row()
                        .h(px(40.0))
                        .child(cell(Some(STATUS_W)).child(status_badge(job.status, false, theme)))
                        .child(cell(None).child(div().truncate().child(job.name.clone())))
                        .child(cell(Some(STAGE_W)).text_color(theme.text_muted).child(div().truncate().child(stage.name.clone())))
                        .child(cell(Some(RUNNER_W)).text_color(theme.text_muted).child(div().truncate().child(job.runner.clone().unwrap_or_else(|| "—".into()))))
                        .child(cell(Some(DURATION_W)).child(duration_text(job.duration, job.started_at)))
                        .child(cell(Some(FINISHED_W)).text_color(theme.text_muted).child(finished)),
                )
        }))
    }
}
