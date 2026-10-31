//! The CI page: replaces everything under the toolbar while it is open.

use gd_ui::{Button, Theme, empty_state};
use gpui::{App, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled, Window, div, prelude::*, px};

use super::pipeline_row::{columns_header, pipeline_row};
use super::pipelines_header::PipelinesHeader;
use super::problem_view::ProblemView;
use super::pipeline_view::PipelineView;
use super::state::{CiStore, CiView};
use crate::components::list;
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct CiPage {
    pub store: Entity<CiStore>,
    pub ws: Entity<Workspace>,
}

impl RenderOnce for CiPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ci = self.store.read(cx);
        let content = match &ci.target {
            None => empty_state("Looking for the account…", cx).flex_1().into_any_element(),
            Some(Err(problem)) => ProblemView { problem: problem.clone(), ws: self.ws.clone() }.into_any_element(),
            Some(Ok(_)) => match ci.view {
                CiView::List => self.pipelines(cx).into_any_element(),
                CiView::Pipeline(id) => PipelineView { store: self.store.clone(), ws: self.ws.clone(), id }.into_any_element(),
            },
        };
        div()
            .id("ci-page")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .when(ci.view == CiView::List || !matches!(ci.target, Some(Ok(_))), |d| {
                d.child(PipelinesHeader { store: self.store.clone(), ws: self.ws.clone() })
            })
            .child(content)
    }
}

impl CiPage {
    fn pipelines(&self, cx: &App) -> impl IntoElement + use<> {
        let ci = self.store.read(cx);
        let theme = Theme::of(cx);
        let count = ci.pipelines.len();
        if count == 0 {
            return match (&ci.list_error, ci.loading) {
                (Some(problem), _) => ProblemView { problem: problem.clone(), ws: self.ws.clone() }.into_any_element(),
                (None, true) => empty_state("Loading pipelines…", cx).flex_1().into_any_element(),
                (None, false) => empty_state("No pipelines match", cx).flex_1().into_any_element(),
            };
        }
        let store = self.store.clone();
        let rows = list("ci-pipelines", count, move |range, window, cx| {
            let ci = store.read(cx);
            let ids: Vec<_> = range.clone().filter_map(|ix| ci.pipelines.get(ix)).map(|p| p.id).collect();
            if ids.iter().any(|id| ci.needs_detail(*id)) {
                let store = store.clone();
                window.defer(cx, move |_, cx| store.update(cx, |s, cx| s.request_details(ids, cx)));
            }
            let ci = store.read(cx);
            range.filter_map(|ix| ci.pipelines.get(ix).map(|p| pipeline_row(ix, p, ci.details.get(&p.id), store.clone(), cx))).collect()
        })
        .track_scroll(&ci.list_scroll)
        .flex_1();
        let store = self.store.clone();
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .px(px(18.0))
            .children(ci.list_error.as_ref().map(|problem| {
                let (title, body) = problem.describe();
                div().px(px(10.0)).py(px(6.0)).text_color(theme.danger).child(format!("{title}. {body}"))
            }))
            .child(columns_header(cx))
            .child(rows)
            .when(ci.more, |d| {
                let label = if ci.loading { "Loading…" } else { "Load more" };
                d.child(div().flex().flex_none().justify_center().py(px(10.0)).child(
                    Button::new("ci-load-more").label(label).disabled(ci.loading).on_click(move |_, _, cx| store.update(cx, |s, cx| s.load_list(true, cx))),
                ))
            })
            .into_any_element()
    }
}
