//! The dropdown of a `ComboBox`: the app's context menu, open under the field.

use gd_ui::{ContextMenu, MenuItem};
use gpui::{
    App, Bounds, Entity, IntoElement, ParentElement, Pixels, RenderOnce, Styled, Window, div,
    point, px,
};

use super::combo_box::{ComboBox, ComboEntry};

/// Longer lists scroll.
const MAX_HEIGHT: f32 = 300.0;

#[derive(IntoElement)]
pub struct ComboMenu {
    pub combo: Entity<ComboBox>,
    pub id: &'static str,
    pub entries: Vec<ComboEntry>,
    /// Text narrows the list: the first entry is what Enter applies.
    pub typed: bool,
    /// The field in window coordinates (the menu is laid out inside it).
    pub field: Bounds<Pixels>,
}

impl RenderOnce for ComboMenu {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let combo = self.combo.clone();
        let mut menu = ContextMenu::new(
            (self.id, 1usize),
            self.field.bottom_left() + point(px(0.0), px(4.0)),
        )
        .selected(self.typed.then_some(0))
        .without_icons()
        .max_height(MAX_HEIGHT)
        .on_dismiss(move |_, cx| combo.update(cx, |c, cx| c.dismiss(cx)));
        if self.entries.is_empty() {
            let note = if self.typed {
                "No match, Enter applies it"
            } else {
                "Nothing to suggest yet."
            };
            menu = menu.item(MenuItem::new(note).disabled(true));
        }
        for entry in self.entries {
            let (combo, value) = (self.combo.clone(), entry.value.clone());
            let mut item = MenuItem::new(entry.value)
                .on_select(move |_, cx| combo.update(cx, |c, cx| c.pick(value.clone(), cx)));
            if let Some(detail) = entry.detail {
                item = item.detail(detail);
            }
            menu = menu.item(item);
        }
        // The menu is laid out inside the field, but its backdrop covers the window: give it the window's content area (inside the
        // client-side decorations inset), where the menu expects its parent. (`anchored` would round that offset.)
        let inset = window.client_inset().unwrap_or_default();
        let view = window.viewport_size();
        div()
            .absolute()
            .left(inset - self.field.origin.x)
            .top(inset - self.field.origin.y)
            .w(view.width - inset * 2.0)
            .h(view.height - inset * 2.0)
            .child(menu)
    }
}
