//! A job in the pipeline graph: status ring, name, duration.

use gd_forge::{Capabilities, Job, Status};
use gd_ui::motion::HOVER_MS;
use gd_ui::{Theme, icon_button};
use gpui::{App, Entity, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled, div, millis, px};

use super::actions::job_actions;
use super::duration::duration_text;
use super::state::CiStore;
use super::status_badge::{look, status_icon};

pub fn job_card(job: &Job, caps: Capabilities, store: Entity<CiStore>, cx: &App) -> impl IntoElement + use<> {
    let theme = Theme::of(cx);
    // A failed job that may fail is a warning, like GitLab's orange.
    let look = if job.allow_failure && job.status == Status::Failed { look(Status::Success, true) } else { look(job.status, false) };
    let id = job.id;
    let group = format!("job-card-{}", id.id);
    let tint = look.tone.color(theme);
    // Opaque at rest and on hover: `pressable`'s translucent wash would fade through the page behind the card.
    div()
        .id(("job-card", job.id.id as usize))
        .cursor_pointer()
        .group(group.clone())
        .flex()
        .items_center()
        .gap(px(8.0))
        .h(px(40.0))
        .px(px(10.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(if look.label == "warning" { tint } else { theme.border })
        .bg(theme.surface_raised)
        .transitions(|t| t.bg(millis(HOVER_MS)))
        .hover_style(|s| s.bg(theme.surface_raised_hover))
        .active_style(|s| s.bg(theme.surface_raised_hover))
        .on_click({
            let store = store.clone();
            move |_, _, cx| store.update(cx, |s, cx| s.open_job(id, cx))
        })
        .child(status_icon(look, 18.0, theme))
        .child(div().flex_1().truncate().child(job.name.clone()))
        .child(div().text_size(px(11.0)).text_color(theme.text_faint).child(duration_text(job.duration, job.started_at)))
        .children(job_actions(job, caps).into_iter().map(|(action, icon, label)| {
            let store = store.clone();
            // Shown while the card is hovered; the click must not also open the job.
            div().id(("job-action-wrap", job.id.id as usize)).invisible().group_hover_style(group.clone(), |s| s.visible()).tooltip(gd_ui::tooltip(label)).child(
                icon_button(("job-action", job.id.id as usize), icon).compact().on_click(move |_, _, cx| {
                    cx.stop_propagation();
                    store.update(cx, |s, cx| s.run_action(action, cx))
                }),
            )
        }))
}
