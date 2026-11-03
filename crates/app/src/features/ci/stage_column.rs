//! One column of the pipeline graph: the stage name over its job cards.

use gd_forge::{Capabilities, Stage};
use gd_ui::Theme;
use gpui::{App, Entity, IntoElement, ParentElement, Styled, div, px};

use super::job_card::job_card;
use super::state::CiStore;
use super::status_badge::{look, status_icon};

pub const COLUMN_WIDTH: f32 = 240.0;

pub fn stage_column(
    stage: &Stage,
    caps: Capabilities,
    store: &Entity<CiStore>,
    cx: &App,
) -> impl IntoElement + use<> {
    let theme = Theme::of(cx);
    div()
        .flex()
        .flex_col()
        .flex_none()
        .w(px(COLUMN_WIDTH))
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .h(px(36.0))
                .px(px(10.0))
                .rounded(px(8.0))
                .bg(theme.ink(0.05))
                .child(status_icon(look(stage.status, false), 16.0, theme))
                .child(
                    div()
                        .flex_1()
                        .truncate()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(stage.name.clone()),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.text_faint)
                        .child(stage.jobs.len().to_string()),
                ),
        )
        .children(
            stage
                .jobs
                .iter()
                .map(|job| job_card(job, caps, store.clone(), cx)),
        )
}
