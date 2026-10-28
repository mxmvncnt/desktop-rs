use gd_ui::Theme;
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

use crate::components::DiffDisplayOptions;
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct DiffPage {
    pub ws: Entity<Workspace>,
}

impl RenderOnce for DiffPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(div().font_weight(gpui::FontWeight::MEDIUM).child("Display"))
            .child(div().w(px(304.0)).child(DiffDisplayOptions { sectioned: false, ws: self.ws }))
            .child(
                div()
                    .pt(px(6.0))
                    .text_size(px(12.0))
                    .text_color(theme.text_faint)
                    .child("These apply to every repository."),
            )
    }
}
