//! One row of the pipelines list (a plain fn: virtualized rows are rebuilt every frame) and the column header above the list.

use gd_forge::{Pipeline, PipelineDetail};
use gd_ui::{Icon, Theme, badge, icon, pressable};
use gpui::{
    App, Div, Entity, IntoElement, ParentElement, SharedString, StatefulInteractiveElement, Styled,
    div, px,
};

use super::duration::duration_text;
use super::stage_dots::stage_dots;
use super::state::CiStore;
use super::status_badge::status_badge;
use crate::utils::text::relative_time;

pub const ROW_HEIGHT: f32 = 60.0;

const STATUS_W: f32 = 112.0;
const REF_W: f32 = 190.0;
const USER_W: f32 = 140.0;
const STAGES_W: f32 = 150.0;
const TIME_W: f32 = 110.0;

/// A cell: fixed width except the flexible title column (`width == None`).
fn cell(width: Option<f32>) -> Div {
    let cell = div()
        .flex()
        .flex_col()
        .justify_center()
        .min_w_0()
        .overflow_hidden();
    match width {
        Some(w) => cell.w(px(w)).flex_none(),
        None => cell.flex_1(),
    }
}

pub fn columns_header(cx: &App) -> Div {
    let theme = Theme::of(cx);
    let label = |text: &'static str, width| cell(width).child(text);
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(12.0))
        .h(px(30.0))
        .mx(px(6.0))
        .px(px(10.0))
        .border_b_1()
        .border_color(theme.border)
        .text_size(px(11.0))
        .text_color(theme.text_faint)
        .child(label("Status", Some(STATUS_W)))
        .child(label("Pipeline", None))
        .child(label("Ref", Some(REF_W)))
        .child(label("Triggerer", Some(USER_W)))
        .child(label("Stages", Some(STAGES_W)))
        .child(label("Duration", Some(TIME_W)))
}

pub fn pipeline_row(
    ix: usize,
    p: &Pipeline,
    detail: Option<&PipelineDetail>,
    store: Entity<CiStore>,
    cx: &App,
) -> impl IntoElement + use<> {
    let theme = Theme::of(cx);
    // The list has no commit title on GitLab, nor the user: the loaded detail has both.
    let shown = detail.map_or(p, |d| &d.pipeline);
    let warnings = detail.is_some_and(PipelineDetail::passed_with_warnings);
    let title = shown
        .title
        .clone()
        .or_else(|| p.workflow.clone())
        .unwrap_or_else(|| format!("Pipeline #{}", p.number));
    let user = shown
        .user
        .as_ref()
        .map(|u| u.name.clone().unwrap_or_else(|| u.username.clone()));
    let created = shown
        .created_at
        .map_or(String::new(), |t| relative_time(t.timestamp()));
    let sha: String = p.sha.chars().take(7).collect();
    let id = p.id;

    pressable(("ci-pipeline", ix), false, cx)
        .flex()
        .items_center()
        .gap(px(12.0))
        .h(px(ROW_HEIGHT))
        .w_full()
        .px(px(10.0))
        .rounded(px(6.0))
        .on_click(move |_, _, cx| store.update(cx, |s, cx| s.open_pipeline(id, cx)))
        .child(
            cell(Some(STATUS_W))
                .items_start()
                .child(status_badge(p.status, warnings, theme)),
        )
        .child(
            cell(None)
                .gap(px(3.0))
                .child(
                    div()
                        .truncate()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(title),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .text_size(px(12.0))
                        .text_color(theme.text_muted)
                        .child(format!("#{}", p.number))
                        .children(shown.workflow.clone().map(|w| div().truncate().child(w)))
                        .children(shown.source.clone().map(|s| badge(s.replace('_', " "), cx))),
                ),
        )
        .child(
            cell(Some(REF_W))
                .gap(px(3.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .child(
                            icon(Icon::GitBranch)
                                .size(px(12.0))
                                .text_color(theme.text_faint),
                        )
                        .child(
                            div()
                                .truncate()
                                .child(SharedString::from(p.git_ref.clone())),
                        ),
                )
                .child(
                    div()
                        .font_family(theme.font_mono.clone())
                        .text_size(px(11.0))
                        .text_color(theme.text_faint)
                        .child(sha),
                ),
        )
        .child(
            cell(Some(USER_W))
                .text_color(theme.text_muted)
                .child(div().truncate().child(user.unwrap_or_else(|| "—".into()))),
        )
        .child(cell(Some(STAGES_W)).child(stage_dots(p.id, detail, cx)))
        .child(
            cell(Some(TIME_W))
                .gap(px(3.0))
                .child(duration_text(p.duration, p.started_at))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.text_faint)
                        .child(created),
                ),
        )
}
