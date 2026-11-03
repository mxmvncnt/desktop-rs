//! A text filter of the pipelines list that is also a dropdown (`CiMenu`): clicking the field lists the choices, typing narrows them, and
//! a pick or Enter applies the filter (the forges filter on exact names, so typed text alone never does). Used for the branch and the user.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    App, Entity, InteractiveElement, IntoElement, ParentElement, Pixels, Point, RenderOnce, Styled,
    Window, canvas, div, point, px,
};
use gpui_elements::editable_text::{
    EditableTextState,
    actions::{Enter, Escape},
};

use super::filter_entries::{Entry, on_enter};
use super::state::{CiStore, Field};
use crate::components::text_field;

#[derive(IntoElement)]
pub struct FilterField {
    pub store: Entity<CiStore>,
    pub field: Field,
    pub input: Entity<EditableTextState>,
    pub placeholder: &'static str,
    pub width: f32,
    /// The choices while the dropdown is open, `None` while it is closed.
    pub entries: Option<Vec<Entry>>,
    /// Set to the bottom left of the field, where the dropdown opens.
    pub anchor: Rc<Cell<Point<Pixels>>>,
}

impl RenderOnce for FilterField {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (field, store, input, anchor) = (self.field, self.store, self.input, self.anchor);
        let field_id = match field {
            Field::Branch => "ci-branch-filter",
            Field::User => "ci-user-filter",
        };
        let values: Vec<String> = self
            .entries
            .iter()
            .flatten()
            .map(|e| e.value.clone())
            .collect();

        // Closing without a pick puts the applied value back in the field.
        let revert = {
            let (store, input) = (store.clone(), input.clone());
            Rc::new(move |cx: &mut App| {
                let applied = store.read(cx).applied(field).to_string();
                input.update(cx, |s, cx| s.emplace(&applied, cx));
                store.update(cx, |s, cx| s.close_menu(cx));
            })
        };

        let enter = {
            let (store, input) = (store.clone(), input.clone());
            move |_: &Enter, _: &mut Window, cx: &mut App| {
                if store.read(cx).menu != Some(field) || !store.read(cx).menu_typed {
                    return;
                }
                cx.stop_propagation();
                let choice = on_enter(&values, input.read(cx).as_str());
                input.update(cx, |s, cx| s.emplace(&choice, cx));
                store.update(cx, |s, cx| s.apply(field, choice, cx));
            }
        };
        let escape = {
            let (store, revert) = (store.clone(), revert.clone());
            move |_: &Escape, _: &mut Window, cx: &mut App| {
                if store.read(cx).menu == Some(field) {
                    cx.stop_propagation();
                    revert(cx);
                }
            }
        };
        // The input handles its own clicks (caret, focus) and stops them, so this listens before it, in the capture phase.
        let open_on_click = {
            let store = store.clone();
            move |_: &gpui::MouseDownEvent, _: &mut Window, cx: &mut App| {
                store.update(cx, |s, cx| s.open_menu(field, cx))
            }
        };
        div()
            .relative()
            .flex()
            .items_center()
            .capture_action(enter)
            .capture_action(escape)
            .child(
                div()
                    .w(px(self.width))
                    .capture_any_mouse_down(open_on_click)
                    .child(
                        canvas(
                            move |bounds, _, _| {
                                anchor.set(bounds.bottom_left() + point(px(0.0), px(4.0)))
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    )
                    .child(text_field(field_id, &input, self.placeholder, cx)),
            )
    }
}
