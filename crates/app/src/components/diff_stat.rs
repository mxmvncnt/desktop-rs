//! Added / deleted line counts, e.g. `+12 −3`.

use gd_ui::Theme;
use gpui::{App, IntoElement, RenderOnce, Window, div, prelude::*, px};

#[derive(IntoElement)]
pub struct DiffStat {
    pub added: u32,
    pub deleted: u32,
}

impl RenderOnce for DiffStat {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        div()
            .flex()
            .flex_none()
            .gap(px(8.0))
            .text_size(px(12.0))
            .font_family(theme.font_mono.clone())
            .child(div().text_color(theme.diff_add).child(format!("+{}", self.added)))
            .child(div().text_color(theme.diff_del).child(format!("−{}", self.deleted)))
    }
}
