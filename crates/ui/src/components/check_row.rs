//! A checkbox row for option lists; clicking the box or the row toggles.

use std::rc::Rc;

use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, prelude::*, px};

use super::{CheckState, Checkbox, pressable};

#[derive(IntoElement)]
pub struct CheckRow {
    id: &'static str,
    text: SharedString,
    checked: bool,
    on_toggle: Rc<dyn Fn(&mut Window, &mut App)>,
}

impl CheckRow {
    pub fn new(
        id: &'static str,
        text: impl Into<SharedString>,
        checked: bool,
        on_toggle: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self { id, text: text.into(), checked, on_toggle: Rc::new(on_toggle) }
    }
}

impl RenderOnce for CheckRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = if self.checked { CheckState::Checked } else { CheckState::Unchecked };
        let on_box = self.on_toggle.clone();
        let on_row = self.on_toggle;
        pressable(self.id, false, cx)
            .flex()
            .items_center()
            .gap(px(10.0))
            .h(px(30.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .text_size(px(13.0))
            .child(Checkbox::new(SharedString::from(format!("{}-check", self.id)), state).on_click(move |_, w, cx| on_box(w, cx)))
            .child(self.text)
            .on_click(move |_, w, cx| on_row(w, cx))
    }
}
