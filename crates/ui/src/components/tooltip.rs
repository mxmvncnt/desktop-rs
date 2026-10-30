//! Small zeron-style tooltip for `.tooltip(gd_ui::tooltip("text"))`.

use gpui::{AnyView, App, AppContext, Context, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, px};

use crate::theme::Theme;

pub struct Tooltip {
    text: SharedString,
}

impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx);
        // GPUI anchors tooltips just below-right of the cursor and exposes no offset. A zero-height
        // anchor with the real box pinned to its bottom edge makes the tooltip sit above the cursor.
        div().relative().h(px(0.0)).child(
            div()
                .absolute()
                .bottom(px(6.0))
                .whitespace_nowrap()
                .px(px(8.0))
                .py(px(5.0))
                .rounded(px(6.0))
                .bg(theme.surface_overlay)
                .border_1()
                .border_color(theme.border_strong)
                .shadow_md()
                .text_size(px(12.0))
                .text_color(theme.text)
                .child(self.text.clone()),
        )
    }
}

/// Builder for `InteractiveElement::tooltip`.
pub fn tooltip(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_, cx| cx.new(|_| Tooltip { text: text.clone() }).into()
}
