//! Layout of the form modals (app settings, repository settings): a sidebar of pages, a title bar
//! with a close button, and the page body. Esc, the close button or a backdrop click calls `on_close`.

use gd_ui::{Callback, Icon, Theme, divider, icon, icon_button, modal, motion, overlay_card, pressable};
use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use gpui_elements::editable_text::actions::Escape;

pub struct NavItem {
    pub label: &'static str,
    pub icon: Icon,
    pub active: bool,
    pub on_click: Callback,
}

#[derive(IntoElement)]
pub struct SettingsShell {
    /// Names the entrance hand-off between overlays (see `motion::handoff`).
    pub id: &'static str,
    pub title: SharedString,
    pub nav: Vec<NavItem>,
    /// Items pinned to the bottom of the sidebar.
    pub nav_bottom: Vec<NavItem>,
    pub body: AnyElement,
    pub on_close: Callback,
}

impl RenderOnce for SettingsShell {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let instant = motion::handoff(&format!("{}-in", self.id).into(), cx);
        let nav = div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .p(px(8.0))
            .w(px(188.0))
            .flex_none()
            .border_r_1()
            .border_color(theme.border)
            .bg(theme.ink(0.02))
            .children(self.nav.into_iter().map(|item| nav_item(item, &theme, cx)))
            .child(div().flex_1())
            .children(self.nav_bottom.into_iter().map(|item| nav_item(item, &theme, cx)));
        let (close, escape, backdrop) = (self.on_close.clone(), self.on_close.clone(), self.on_close);
        let title_bar = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .h(px(52.0))
            .pl(px(24.0))
            .pr(px(12.0))
            .child(div().text_size(px(15.0)).font_weight(gpui::FontWeight::SEMIBOLD).child(self.title))
            .child(icon_button("settings-close", Icon::Close).compact().on_click(move |_, w, cx| close(w, cx)));
        let card = overlay_card(cx)
            .id("settings-card")
            .occlude()
            // A focused text input turns Esc into its own action: capture it to close the modal.
            .capture_action(move |_: &Escape, w, cx| {
                cx.stop_propagation();
                escape(w, cx)
            })
            .w(px(740.0))
            .h(px(560.0))
            .flex()
            .child(nav)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(title_bar)
                    .child(divider(cx))
                    .child(
                        // The padding is inside the scroll area, so scrolled content runs up to the divider and the bottom edge.
                        div().id("settings-scroll").flex_1().min_h_0().overflow_y_scroll().child(div().p(px(24.0)).child(self.body)),
                    ),
            );
        modal(card, Some(backdrop), instant)
    }
}

fn nav_item(item: NavItem, theme: &Theme, cx: &mut App) -> impl IntoElement + use<> {
    let color = if item.active { theme.text } else { theme.text_muted };
    pressable(item.label, item.active, cx)
        .flex()
        .items_center()
        .gap(px(10.0))
        .h(px(30.0))
        .px(px(10.0))
        .rounded(px(gd_ui::theme::CONTROL_RADIUS))
        .text_size(px(13.0))
        .text_color(color)
        .child(icon(item.icon).size(px(15.0)).text_color(if item.active { theme.text } else { theme.text_faint }))
        .child(item.label)
        .on_click(move |_, w, cx| (item.on_click)(w, cx))
}
