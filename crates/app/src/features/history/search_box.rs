//! Search input above the commit list.

use gd_ui::divider;
use gpui::{App, Entity, IntoElement, RenderOnce, Window, div, prelude::*, px};
use gpui_elements::editable_text::EditableTextState;

use crate::shared::text_field::text_field;

#[derive(IntoElement)]
pub(super) struct SearchBox {
    input: Entity<EditableTextState>,
}

impl SearchBox {
    pub fn new(input: Entity<EditableTextState>) -> Self {
        Self { input }
    }
}

impl RenderOnce for SearchBox {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex_none()
            .child(div().p(px(8.0)).child(text_field("history-search", &self.input, "Search commits", cx)))
            .child(divider(cx))
    }
}
