use gd_ui::{Tabs, Theme};
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

use crate::features::diff::CheckOptionRow;
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct DiffPage {
    split: bool,
    wrap: bool,
    hide_whitespace: bool,
    ws: Entity<Workspace>,
}

impl DiffPage {
    pub fn new(split: bool, wrap: bool, hide_whitespace: bool, ws: Entity<Workspace>) -> Self {
        Self { split, wrap, hide_whitespace, ws }
    }

    fn display_tabs(&self) -> impl IntoElement {
        let ws = self.ws.clone();
        div().w(px(300.0)).pb(px(6.0)).child(
            Tabs::new("settings-diff-display", usize::from(self.split))
                .tab("Unified", None)
                .tab("Split", None)
                .on_select(move |ix, _, cx| ws.update(cx, |this, cx| this.set_diff_split(ix == 1, cx))),
        )
    }
}

impl RenderOnce for DiffPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let (wrap, hide_ws) = (self.wrap, self.hide_whitespace);
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(div().font_weight(gpui::FontWeight::MEDIUM).child("Display"))
            .child(self.display_tabs())
            .child(CheckOptionRow::new("settings-diff-wrap", "Wrap long lines", wrap, self.ws.clone(), move |this, cx| {
                this.set_diff_wrap(!wrap, cx)
            }))
            .child(CheckOptionRow::new("settings-diff-ws", "Hide whitespace changes", hide_ws, self.ws, move |this, cx| {
                this.set_ignore_whitespace(!hide_ws, cx)
            }))
            .child(
                div()
                    .pt(px(6.0))
                    .text_size(px(12.0))
                    .text_color(theme.text_faint)
                    .child("These apply to every repository."),
            )
    }
}
