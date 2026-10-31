//! The mini graph of a pipeline row: one ring per stage, with the stage and its status in a tooltip.

use gd_forge::{PipelineDetail, PipelineId};
use gd_ui::Theme;
use gpui::{App, Div, InteractiveElement, StatefulInteractiveElement, Styled, div, prelude::*, px};

use super::status_badge::{look, status_icon};

/// Stages of `detail`; faint dashes until it is loaded.
pub fn stage_dots(pipeline: PipelineId, detail: Option<&PipelineDetail>, cx: &App) -> Div {
    let theme = Theme::of(cx);
    let Some(detail) = detail else {
        return div().text_color(theme.text_faint).child("—");
    };
    div().flex().items_center().gap(px(4.0)).children(detail.stages.iter().enumerate().map(|(ix, stage)| {
        let look = look(stage.status, false);
        div()
            .id(("stage-dot", pipeline.0 as usize * 32 + ix))
            .tooltip(gd_ui::tooltip(format!("{}: {}", stage.name, look.label)))
            .child(status_icon(look, 18.0, theme))
    }))
}
