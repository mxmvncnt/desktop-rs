//! Modal dialog: scrim, centered card, title, body, custom content, and
//! Cancel + confirm buttons. Stateless like `ContextMenu`.

use std::rc::Rc;

use gpui::{App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce, SharedString, Styled, Window, deferred, div, px};

use super::{Button, ButtonStyle, overlay_card};
use crate::motion;
use crate::theme::{Theme, ink, scrim};

type Callback = Rc<dyn Fn(&mut Window, &mut App)>;

/// Modal confirmation dialog: scrim, centered card, title, body, and
/// Cancel + confirm buttons (confirm is primary, or red with [`Dialog::danger`]).
#[derive(IntoElement)]
pub struct Dialog {
    id: ElementId,
    title: SharedString,
    body: Option<SharedString>,
    detail: Option<SharedString>,
    content: Option<gpui::AnyElement>,
    confirm_label: SharedString,
    cancel_label: SharedString,
    danger: bool,
    confirm_disabled: bool,
    width: f32,
    on_confirm: Option<Callback>,
    on_cancel: Option<Callback>,
}

impl Dialog {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            body: None,
            detail: None,
            content: None,
            confirm_label: "OK".into(),
            cancel_label: "Cancel".into(),
            danger: false,
            confirm_disabled: false,
            width: 440.0,
            on_confirm: None,
            on_cancel: None,
        }
    }
    /// Card width in px (default 440).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Custom content (inputs, lists) between the text and the buttons.
    pub fn content(mut self, c: impl IntoElement) -> Self {
        self.content = Some(c.into_any_element());
        self
    }
    pub fn confirm_disabled(mut self, d: bool) -> Self {
        self.confirm_disabled = d;
        self
    }
    pub fn body(mut self, b: impl Into<SharedString>) -> Self {
        self.body = Some(b.into());
        self
    }
    /// Smaller, muted secondary text under the body.
    pub fn detail(mut self, d: impl Into<SharedString>) -> Self {
        self.detail = Some(d.into());
        self
    }
    pub fn confirm_label(mut self, l: impl Into<SharedString>) -> Self {
        self.confirm_label = l.into();
        self
    }
    pub fn cancel_label(mut self, l: impl Into<SharedString>) -> Self {
        self.cancel_label = l.into();
        self
    }
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }
    pub fn on_confirm(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_confirm = Some(Rc::new(f));
        self
    }
    /// Called for Cancel, a backdrop click, and (by the owner) Esc.
    pub fn on_cancel(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Dialog {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let instant = motion::handoff(&self.id, cx);
        let cancel = self.on_cancel;
        let confirm = self.on_confirm;

        let mut cancel_btn = Button::new("dialog-cancel").label(self.cancel_label);
        if let Some(f) = cancel.clone() {
            cancel_btn = cancel_btn.on_click(move |_, w, cx| f(w, cx));
        }
        let mut confirm_btn = Button::new("dialog-confirm")
            .label(self.confirm_label)
            .style(if self.danger { ButtonStyle::Danger } else { ButtonStyle::Primary });
        confirm_btn = confirm_btn.disabled(self.confirm_disabled);
        if let Some(f) = confirm {
            confirm_btn = confirm_btn.on_click(move |_, w, cx| f(w, cx));
        }

        let card = overlay_card(cx)
            .id(self.id)
            .occlude()
            .w(px(self.width))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .px(px(20.0))
                    .pt(px(18.0))
                    .pb(px(16.0))
                    .child(
                        div()
                            .text_size(px(15.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(self.title),
                    )
                    .children(self.body.map(|b| {
                        div().text_size(px(13.0)).text_color(theme.text_muted).child(b)
                    }))
                    .children(self.detail.map(|d| {
                        div().text_size(px(12.0)).text_color(theme.text_faint).child(d)
                    }))
                    .children(self.content),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.0))
                    .px(px(16.0))
                    .py(px(12.0))
                    .border_t_1()
                    .border_color(theme.border)
                    .bg(ink(0.02))
                    .child(cancel_btn)
                    .child(confirm_btn),
            );

        let mut backdrop = div().id("dialog-backdrop").occlude().absolute().inset_0().bg(scrim(0.5));
        if let Some(f) = cancel {
            backdrop = backdrop.on_mouse_down(MouseButton::Left, move |_, w, cx| f(w, cx));
        }
        deferred(
            div()
                .absolute()
                .inset_0()
                .child(motion::scrim_in("dialog-scrim-in", backdrop, instant))
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(motion::dialog_in("dialog-in", card, instant)),
                ),
        )
    }
}
