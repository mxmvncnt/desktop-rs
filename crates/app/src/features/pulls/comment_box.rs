//! A text box with its buttons, for a comment on the conversation, on lines, or a reply.

use gd_ui::{Button, Callback, Theme};
use gpui::{App, ElementId, Entity, IntoElement, ParentElement, Styled, div, px};
use gpui_elements::editable_text::EditableTextState;

use crate::components::text_area;

/// Text areas need a definite width to wrap (see ARCHITECTURE, GPUI-CE gotchas).
pub const BOX_W: f32 = 620.0;

pub struct CommentBox {
    pub id: &'static str,
    pub input: Entity<EditableTextState>,
    pub placeholder: String,
    pub action: &'static str,
    pub posting: bool,
    pub on_post: Callback,
    pub on_cancel: Option<Callback>,
}

impl CommentBox {
    pub fn build(self, cx: &App) -> impl IntoElement + use<> {
        let theme = Theme::of(cx);
        let empty = self.input.read(cx).as_str().trim().is_empty();
        let (on_post, on_cancel) = (self.on_post, self.on_cancel);
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .w(px(BOX_W))
            .child(
                text_area(self.id, &self.input, &self.placeholder, cx)
                    .w(px(BOX_W))
                    .h(px(76.0))
                    .bg(theme.input_bg),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.0))
                    .children(on_cancel.map(|cancel| {
                        Button::new(ElementId::Name(format!("{}-cancel", self.id).into()))
                            .label("Cancel")
                            .ghost()
                            .on_click(move |_, w, cx| cancel(w, cx))
                    }))
                    .child(
                        Button::new(ElementId::Name(format!("{}-post", self.id).into()))
                            .primary()
                            .label(if self.posting {
                                "Sending…"
                            } else {
                                self.action
                            })
                            .disabled(empty || self.posting)
                            .on_click(move |_, w, cx| on_post(w, cx)),
                    ),
            )
    }
}
