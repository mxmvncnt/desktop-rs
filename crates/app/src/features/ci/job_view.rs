//! A job: breadcrumb, then the log next to the job's details.

use gd_forge::JobId;
use gd_ui::{Icon, Theme, empty_state, icon, icon_button};
use gpui::{App, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled, Window, div, px};

use super::actions::job_actions;
use super::job_sidebar::JobSidebar;
use super::log_view::LogView;
use super::state::{CiStore, CiView};

#[derive(IntoElement)]
pub struct JobView {
    pub store: Entity<CiStore>,
    pub id: JobId,
}

impl RenderOnce for JobView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let ci = self.store.read(cx);
        let Some(job) = ci.job(self.id).cloned() else {
            return empty_state("Loading job…", cx).flex_1().into_any_element();
        };
        let actions = ci.ready().map(|t| job_actions(&job, t.forge.capabilities())).unwrap_or_default();
        let number = ci.details[&self.id.pipeline].pipeline.number;
        let stage = ci.job(self.id).and_then(|_| ci.details[&self.id.pipeline].stages.iter().find(|s| s.jobs.iter().any(|j| j.id == self.id))).map(|s| s.name.clone());
        let (to_list, to_pipeline, back) = (self.store.clone(), self.store.clone(), self.store.clone());
        let chevron = || icon(Icon::ChevronRight).size(px(12.0)).text_color(theme.text_faint);
        let link = |id: &'static str, text: String| div().id(id).cursor_pointer().hover_style(|s| s.underline()).child(text);
        let breadcrumb = div()
            .flex()
            .items_center()
            .flex_none()
            .gap(px(6.0))
            .px(px(24.0))
            .py(px(14.0))
            .text_color(theme.text_muted)
            .child(icon_button("ci-back", Icon::ArrowLeft).on_click(move |_, _, cx| back.update(cx, |s, cx| s.back(cx))))
            .child(link("ci-crumb-list", "Pipelines".into()).on_click(move |_, _, cx| to_list.update(cx, |s, cx| {
                s.view = CiView::List;
                s.log_load.cancel();
                cx.notify()
            })))
            .child(chevron())
            .child(link("ci-crumb-pipeline", format!("#{number}")).on_click(move |_, _, cx| to_pipeline.update(cx, |s, cx| s.back(cx))))
            .child(chevron())
            .child(div().text_color(theme.text).child(job.name.clone()));
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(breadcrumb)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(LogView { store: self.store.clone(), job: self.id })
                    .child(JobSidebar { store: self.store.clone(), actions, acting: ci.acting, job, stage: stage.unwrap_or_default() }),
            )
            .into_any_element()
    }
}
