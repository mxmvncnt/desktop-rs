//! A compact dropdown. Like a native select, the open menu puts the current choice over the control.

use std::rc::Rc;

use gd_ui::{Callback, Icon, Theme, overlay_card};
use gpui::{
    App, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, deferred, div, px,
};

use super::BranchRow;

const ROW_HEIGHT: f32 = 28.0;
/// Width of the control and of its menu: fits "GitHub Enterprise" and the chevron.
const WIDTH: f32 = 220.0;

#[derive(IntoElement)]
pub struct Select {
    id: &'static str,
    value: SharedString,
    open: bool,
    on_toggle: Callback,
    options: Vec<(SharedString, Icon, bool, Callback)>,
}

impl Select {
    /// `open` comes from the owner's state; `on_toggle` flips it (the control and the backdrop call it).
    pub fn new(
        id: &'static str,
        value: impl Into<SharedString>,
        open: bool,
        on_toggle: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id,
            value: value.into(),
            open,
            on_toggle: Rc::new(on_toggle),
            options: Vec::new(),
        }
    }

    /// A choice; the one labeled like the value is highlighted. A disabled one is greyed out.
    pub fn option(
        mut self,
        label: impl Into<SharedString>,
        icon: Icon,
        enabled: bool,
        on_pick: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.options
            .push((label.into(), icon, enabled, Rc::new(on_pick)));
        self
    }
}

impl RenderOnce for Select {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let selected = self
            .options
            .iter()
            .position(|o| o.0 == self.value)
            .unwrap_or(0);
        let menu = self.open.then(|| {
            let rows = self.options.into_iter().enumerate().map(
                |(ix, (label, icon, enabled, on_pick))| {
                    BranchRow::new(self.id, ix, label, move |w, cx| on_pick(w, cx))
                        .icon(icon)
                        .height(ROW_HEIGHT)
                        .selected(ix == selected)
                        .disabled(!enabled)
                },
            );
            let close = self.on_toggle.clone();
            // The backdrop catches the click that dismisses the menu, wherever it lands (even on the control).
            deferred(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .child(
                        div()
                            .absolute()
                            .top(px(-3000.0))
                            .left(px(-3000.0))
                            .size(px(6000.0))
                            .occlude()
                            .on_mouse_down(MouseButton::Left, move |_, w, cx| close(w, cx)),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(-(2.0 + selected as f32 * ROW_HEIGHT)))
                            .left_0()
                            .w(px(WIDTH))
                            .child(overlay_card(cx).occlude().p(px(4.0)).children(rows)),
                    ),
            )
            .priority(2)
        });
        let toggle = self.on_toggle;
        div()
            .relative()
            .w(px(WIDTH))
            .child(
                // Styled like the text fields: the value on the left, the chevron on the right.
                div()
                    .id(self.id)
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(32.0))
                    .px(px(10.0))
                    .rounded(px(gd_ui::theme::CONTROL_RADIUS + 1.0))
                    .bg(theme.input_bg)
                    .border_1()
                    .border_color(theme.border)
                    .cursor_pointer()
                    .hover_style(|s| s.border_color(theme.text_faint))
                    .child(self.value)
                    .child(
                        gd_ui::icon(Icon::ChevronDown)
                            .size(px(14.0))
                            .text_color(theme.text_muted),
                    )
                    .on_click(move |_, w, cx| toggle(w, cx)),
            )
            .children(menu)
    }
}
