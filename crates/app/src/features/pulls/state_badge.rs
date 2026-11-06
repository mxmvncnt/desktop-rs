//! The pill of a pull request's state: open, draft, merged or closed.

use gd_forge::{PullRequest, PullState};
use gd_ui::{Icon, Theme, icon};
use gpui::{ColorExt, Div, FontWeight, ParentElement, Styled, div, px};

pub fn state_badge(p: &PullRequest, theme: &Theme) -> Div {
    let (label, glyph, color) = match (p.state, p.draft) {
        (PullState::Open, true) => ("Draft", Icon::PullRequest, theme.text_muted),
        (PullState::Open, false) => ("Open", Icon::PullRequest, theme.success),
        (PullState::Merged, _) => ("Merged", Icon::Fork, theme.accent),
        (PullState::Closed, _) => ("Closed", Icon::Close, theme.danger),
    };
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(5.0))
        .h(px(22.0))
        .px(px(9.0))
        .rounded_full()
        .bg(color.opacity(0.12))
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(color)
        .child(icon(glyph).size(px(12.0)).text_color(color))
        .child(label)
}
