//! A job: the breadcrumbs, then the log next to the job's details.

use gd_forge::JobId;
use gd_ui::{Theme, empty_state};
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div};

use super::actions::job_actions;
use super::breadcrumbs::Breadcrumbs;
use super::job_sidebar::JobSidebar;
use super::log_view::LogView;
use super::state::CiStore;
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct JobView {
    pub store: Entity<CiStore>,
    pub ws: Entity<Workspace>,
    pub id: JobId,
}

impl RenderOnce for JobView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let ci = self.store.read(cx);
        let Some(job) = ci.job(self.id).cloned() else {
            return empty_state("Loading job…", cx).flex_1().into_any_element();
        };
        let actions = ci
            .ready()
            .map(|t| job_actions(&job, t.forge.capabilities()))
            .unwrap_or_default();
        let number = ci.details[&self.id.pipeline].pipeline.number;
        let stage = ci
            .job(self.id)
            .and_then(|_| {
                ci.details[&self.id.pipeline]
                    .stages
                    .iter()
                    .find(|s| s.jobs.iter().any(|j| j.id == self.id))
            })
            .map(|s| s.name.clone());
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(Breadcrumbs {
                store: self.store.clone(),
                pipeline: number,
                job: Some(job.name.clone()),
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(LogView {
                        store: self.store.clone(),
                        ws: self.ws.clone(),
                        job: self.id,
                    })
                    .child(JobSidebar {
                        store: self.store.clone(),
                        actions,
                        acting: ci.acting,
                        job,
                        stage: stage.unwrap_or_default(),
                    }),
            )
            .into_any_element()
    }
}
