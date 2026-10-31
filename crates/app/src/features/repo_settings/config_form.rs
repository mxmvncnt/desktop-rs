//! Add / edit form of one config entry: the key (fixed while editing) and its value.

use gd_ui::{Button, Theme};
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

use super::state::{RepoSettingsInputs, RepoSettingsStore};
use crate::components::FieldRow;

#[derive(IntoElement)]
pub struct ConfigForm {
    /// Editing an entry: its key cannot change.
    pub editing: bool,
    pub inputs: RepoSettingsInputs,
    pub error: Option<String>,
    pub store: Entity<RepoSettingsStore>,
}

impl RenderOnce for ConfigForm {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let (save, cancel) = (self.store.clone(), self.store);
        let inp = &self.inputs;
        let key = if self.editing {
            div().flex().flex_col().gap(px(4.0)).child(div().text_size(px(12.0)).text_color(theme.text_muted).child("Key")).child(
                div().font_family(theme.font_mono.clone()).text_size(px(12.0)).child(inp.config_key.read(cx).as_str().to_string()),
            )
            .into_any_element()
        } else {
            FieldRow::new("Key (section.key or section.subsection.key)", "config-key", &inp.config_key, "core.editor").into_any_element()
        };
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(12.0))
            .rounded(px(gd_ui::theme::CONTROL_RADIUS + 2.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.ink(0.03))
            .child(key)
            .child(FieldRow::new("Value", "config-value", &inp.config_value, ""))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(Button::new("config-save").primary().compact().label(if self.editing { "Save" } else { "Add" }).on_click(
                        move |_, _, cx| save.update(cx, |this, cx| this.save_config(cx)),
                    ))
                    .child(Button::new("config-cancel").ghost().compact().label("Cancel").on_click(move |_, _, cx| {
                        cancel.update(cx, |this, cx| this.close_edit(cx))
                    }))
                    .children(self.error.map(|e| div().text_size(px(12.0)).text_color(theme.danger).child(e))),
            )
    }
}
