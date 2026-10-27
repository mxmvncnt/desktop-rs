//! Small style building blocks: icons, hairline dividers, the pressable
//! row/ghost surface, pill badges and the overlay card.

use gpui::{App, Div, ElementId, SharedString, Stateful, Styled, Svg, div, millis, prelude::*, px, svg};

use crate::assets::Icon;
use crate::motion::HOVER_MS;
use crate::theme::Theme;

pub fn icon(icon: Icon) -> Svg {
    svg().path(icon.path()).size(px(16.0)).flex_none()
}

/// Horizontal hairline divider.
pub fn divider(cx: &App) -> Div {
    div().h(px(1.0)).w_full().flex_none().bg(Theme::of(cx).border)
}

/// Vertical hairline divider.
pub fn vdivider(cx: &App) -> Div {
    div().w(px(1.0)).h_full().flex_none().bg(Theme::of(cx).border)
}

/// An interactive surface with zeron's hover/active ink washes, transitioned.
/// Base building block for rows and ghost buttons.
pub fn pressable(id: impl Into<ElementId>, selected: bool, cx: &App) -> Stateful<Div> {
    let theme = Theme::of(cx);
    let rest = if selected {
        theme.element_active
    } else {
        theme.ink(0.0)
    };
    let hover = if selected {
        theme.element_active
    } else {
        theme.element_hover
    };
    let active = theme.element_active;
    div()
        .id(id)
        .bg(rest)
        .transitions(|t| t.bg(millis(HOVER_MS)))
        .hover_style(move |s| s.bg(hover))
        .active_style(move |s| s.bg(active))
        .cursor_pointer()
}

/// A small pill badge (counts, ahead/behind).
pub fn badge(text: impl Into<SharedString>, cx: &App) -> Div {
    let theme = Theme::of(cx);
    div()
        .flex_none()
        .px(px(6.0))
        .h(px(18.0))
        .flex()
        .items_center()
        .rounded_full()
        .bg(theme.ink(0.08))
        .text_size(px(11.0))
        .text_color(theme.text_muted)
        .child(text.into())
}

/// Centered faint placeholder text ("No matches"); callers size it
/// (`flex_1()`, `h_full()`).
pub fn empty_state(text: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(12.0))
        .text_color(Theme::of(cx).text_faint)
        .child(text.into())
}

/// The floating card used by popovers, menus and dialogs.
pub fn overlay_card(cx: &App) -> Div {
    let theme = Theme::of(cx);
    div()
        .bg(theme.surface_overlay)
        .border_1()
        .border_color(theme.border_strong)
        .rounded(px(crate::theme::PANEL_RADIUS + 2.0))
        .shadow_lg()
        .overflow_hidden()
}
