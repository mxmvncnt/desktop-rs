//! A one-line success / error note shown under a section.

use gd_ui::Theme;
use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px};

#[derive(IntoElement, Clone, Debug)]
pub struct StatusText {
    text: SharedString,
    error: bool,
}

impl StatusText {
    pub fn ok(text: impl Into<SharedString>) -> Self {
        Self { text: text.into(), error: false }
    }

    pub fn err(text: impl Into<SharedString>) -> Self {
        Self { text: text.into(), error: true }
    }
}

impl RenderOnce for StatusText {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let color = if self.error { theme.danger } else { theme.success };
        div().text_size(px(12.0)).text_color(color).child(self.text)
    }
}
