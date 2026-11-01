//! A text filter of the pipelines list that is also a dropdown: clicking the field or its chevron lists the choices, typing narrows them, and
//! a pick or Enter applies the filter (the forges filter on exact names, so typed text alone never does). Used for the branch and the user.

use std::rc::Rc;
use std::sync::Arc;

use gd_ui::{Icon, Theme, icon_button, overlay_card, pressable};
use gpui::{
    App, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce, StatefulInteractiveElement, Styled, Window, deferred,
    div, px, uniform_list,
};
use gpui_elements::editable_text::{
    EditableTextState,
    actions::{Enter, Escape},
};

use super::filter_entries::{Entry, on_enter};
use super::state::{CiStore, Field};
use crate::components::text_field;

const ROW_HEIGHT: f32 = 28.0;
const MAX_ROWS: usize = 10;
const MENU_WIDTH: f32 = 280.0;

#[derive(IntoElement)]
pub struct FilterField {
    pub store: Entity<CiStore>,
    pub field: Field,
    pub input: Entity<EditableTextState>,
    pub placeholder: &'static str,
    pub width: f32,
    /// The choices while the dropdown is open, `None` while it is closed.
    pub entries: Option<Vec<Entry>>,
    /// The text narrowing them: empty when nothing was typed since it opened.
    pub typed: String,
}

impl RenderOnce for FilterField {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (field, store, input) = (self.field, self.store, self.input);
        let (field_id, menu_id, chevron_id) = match field {
            Field::Branch => ("ci-branch-filter", "ci-branch-menu", "ci-branch-chevron"),
            Field::User => ("ci-user-filter", "ci-user-menu", "ci-user-chevron"),
        };
        let open = self.entries.is_some();
        let entries = Arc::new(self.entries.unwrap_or_default());
        let values: Vec<String> = entries.iter().map(|e| e.value.clone()).collect();
        let typed = !self.typed.is_empty();

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
            move |_: &gpui::MouseDownEvent, _: &mut Window, cx: &mut App| store.update(cx, |s, cx| s.open_menu(field, cx))
        };
        let toggle = {
            let store = store.clone();
            move |_: &gpui::MouseDownEvent, _: &mut Window, cx: &mut App| {
                cx.stop_propagation();
                if open {
                    revert(cx)
                } else {
                    store.update(cx, |s, cx| s.open_menu(field, cx))
                }
            }
        };

        let menu = open.then(|| {
            let theme = Theme::of(cx);
            let body = if entries.is_empty() {
                let note = if typed { "No match. Enter applies the text as typed." } else { "Nothing to suggest yet." };
                div().h(px(ROW_HEIGHT)).px(px(10.0)).flex().items_center().text_color(theme.text_faint).child(note).into_any_element()
            } else {
                let count = entries.len();
                let (store, input) = (store.clone(), input.clone());
                uniform_list(menu_id, count, move |range, _, cx| {
                    let theme = Theme::of(cx);
                    range
                        .map(|ix| {
                            let entry = &entries[ix];
                            let (store, input, value) = (store.clone(), input.clone(), entry.value.clone());
                            // The first one is what Enter applies once something is typed.
                            pressable((menu_id, ix), typed && ix == 0, cx)
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .h(px(ROW_HEIGHT))
                                .px(px(10.0))
                                .rounded(px(gd_ui::theme::CONTROL_RADIUS))
                                .child(div().flex_none().child(entry.value.clone()))
                                .children(entry.detail.clone().map(|d| div().flex_1().truncate().text_size(px(12.0)).text_color(theme.text_faint).child(d)))
                                .on_click(move |_, _, cx| {
                                    input.update(cx, |s, cx| s.emplace(&value, cx));
                                    store.update(cx, |s, cx| s.apply(field, value.clone(), cx));
                                })
                        })
                        .collect()
                })
                .h(px(count.min(MAX_ROWS) as f32 * ROW_HEIGHT))
                .into_any_element()
            };
            // A click in the list must not reach the workspace, which closes the dropdown on any other click.
            deferred(
                div().absolute().top(px(36.0)).left_0().w(px(MENU_WIDTH)).child(
                    overlay_card(cx).occlude().p(px(4.0)).on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()).child(body),
                ),
            )
            .priority(2)
        });

        div()
            .relative()
            .flex()
            .items_center()
            .gap(px(4.0))
            .capture_action(enter)
            .capture_action(escape)
            .child(div().w(px(self.width)).capture_any_mouse_down(open_on_click).child(text_field(field_id, &input, self.placeholder, cx)))
            .child(div().capture_any_mouse_down(toggle).child(icon_button(chevron_id, Icon::ChevronDown)))
            .children(menu)
    }
}
