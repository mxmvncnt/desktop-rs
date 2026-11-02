//! The dropdown of a filter field: the app's context menu, open under the field while the field has the dropdown.

use gd_ui::{ContextMenu, MenuItem};
use gpui::{App, Entity, IntoElement, Pixels, Point, RenderOnce, Window};

use super::filter_entries::Entry;
use super::state::{CiInputs, CiStore, Field};
use crate::workspace::Workspace;

/// Longer lists scroll.
const MAX_HEIGHT: f32 = 300.0;

#[derive(IntoElement)]
pub struct CiMenu {
    pub ws: Entity<Workspace>,
    pub store: Entity<CiStore>,
    pub inputs: CiInputs,
    pub field: Field,
    pub entries: Vec<Entry>,
    /// Something was typed: the first entry is what Enter applies.
    pub typed: bool,
    pub position: Point<Pixels>,
}

impl RenderOnce for CiMenu {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (field, ws) = (self.field, self.ws);
        let mut menu = ContextMenu::new(match field {
            Field::Branch => "ci-branch-menu",
            Field::User => "ci-user-menu",
        }, self.position)
        .selected(self.typed.then_some(0))
        .without_icons()
        .max_height(MAX_HEIGHT)
        .on_dismiss(move |_, cx| ws.update(cx, |ws, cx| ws.close_ci_menu(cx)));
        if self.entries.is_empty() {
            let note = if self.typed { "No match, Enter applies it" } else { "Nothing to suggest yet." };
            return menu.item(MenuItem::new(note).disabled(true));
        }
        for entry in self.entries {
            let (store, value) = (self.store.clone(), entry.value.clone());
            let input = match field {
                Field::Branch => self.inputs.branch.clone(),
                Field::User => self.inputs.user.clone(),
            };
            let mut item = MenuItem::new(entry.value).on_select(move |_, cx| {
                input.update(cx, |s, cx| s.emplace(&value, cx));
                store.update(cx, |s, cx| s.apply(field, value.clone(), cx));
            });
            if let Some(detail) = entry.detail {
                item = item.detail(detail);
            }
            menu = menu.item(item);
        }
        menu
    }
}
