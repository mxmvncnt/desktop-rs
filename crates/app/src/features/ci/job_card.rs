//! A job in the pipeline graph: status ring, name, duration.

use gd_forge::{Job, Status};
use gd_ui::{Theme, pressable};
use gpui::{App, Entity, IntoElement, ParentElement, StatefulInteractiveElement, Styled, div, px};

use super::duration::duration_text;
use super::state::CiStore;
use super::status_badge::{look, status_icon};

pub fn job_card(job: &Job, store: Entity<CiStore>, cx: &App) -> impl IntoElement + use<> {
    let theme = Theme::of(cx);
    // A failed job that may fail is a warning, like GitLab's orange.
    let look = if job.allow_failure && job.status == Status::Failed { look(Status::Success, true) } else { look(job.status, false) };
    let id = job.id;
    let tint = look.tone.color(theme);
    pressable(("job-card", job.id.id as usize), false, cx)
        .flex()
        .items_center()
        .gap(px(8.0))
        .h(px(40.0))
        .px(px(10.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(if look.label == "warning" { tint } else { theme.border })
        .bg(theme.surface)
        .on_click(move |_, _, cx| store.update(cx, |s, cx| s.open_job(id, cx)))
        .child(status_icon(look, 18.0, theme))
        .child(div().flex_1().truncate().child(job.name.clone()))
        .child(div().text_size(px(11.0)).text_color(theme.text_faint).child(duration_text(job.duration, job.started_at)))
}
