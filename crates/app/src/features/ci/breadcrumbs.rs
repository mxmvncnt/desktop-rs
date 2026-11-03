//! The breadcrumb bar of the pipeline and job pages: back arrow, "Pipelines", the pipeline number and, on a job, its name.
//! One component with a fixed height, so the bar is at the same place on both pages.

use gd_ui::{Icon, Theme, icon, icon_button};
use gpui::{
    App, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use super::state::CiStore;

#[derive(IntoElement)]
pub struct Breadcrumbs {
    pub store: Entity<CiStore>,
    pub pipeline: u64,
    /// Name of the open job; `None` on the pipeline page.
    pub job: Option<String>,
}

impl RenderOnce for Breadcrumbs {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let (back, to_list, to_pipeline) =
            (self.store.clone(), self.store.clone(), self.store.clone());
        let chevron = || {
            icon(Icon::ChevronRight)
                .size(px(12.0))
                .text_color(theme.text_faint)
        };
        let link = |id: &'static str, text: String| {
            div()
                .id(id)
                .cursor_pointer()
                .hover_style(|s| s.underline())
                .child(text)
        };
        let pipeline = format!("#{}", self.pipeline);
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(6.0))
            .h(px(56.0))
            .px(px(24.0))
            .text_color(theme.text_muted)
            .child(
                icon_button("ci-back", Icon::ArrowLeft)
                    .on_click(move |_, _, cx| back.update(cx, |s, cx| s.back(cx))),
            )
            .child(
                link("ci-crumb-list", "Pipelines".into())
                    .on_click(move |_, _, cx| to_list.update(cx, |s, cx| s.show_list(cx))),
            )
            .child(chevron())
            .child(match &self.job {
                Some(_) => link("ci-crumb-pipeline", pipeline)
                    .on_click(move |_, _, cx| to_pipeline.update(cx, |s, cx| s.back(cx)))
                    .into_any_element(),
                None => div()
                    .text_color(theme.text)
                    .child(pipeline)
                    .into_any_element(),
            })
            .children(self.job.map(|name| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(chevron())
                    .child(div().text_color(theme.text).child(name))
            }))
    }
}
