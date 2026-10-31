//! Repository settings → Git config: the local config (`.git/config`) as key/value rows.
//! A multi-valued key is several rows; each row edits or removes its own value.

use gd_git::ConfigEntry;
use gd_ui::{Button, Theme};
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px};

use super::config_form::ConfigForm;
use super::state::{Edit, RepoSettingsInputs, RepoSettingsStore};

#[derive(IntoElement)]
pub struct ConfigPage {
    pub entries: Vec<ConfigEntry>,
    pub edit: Option<Edit>,
    pub error: Option<String>,
    pub notice: Option<&'static str>,
    pub inputs: RepoSettingsInputs,
    pub store: Entity<RepoSettingsStore>,
}

impl ConfigPage {
    fn row(&self, ix: usize, entry: &ConfigEntry, theme: &Theme) -> impl IntoElement + use<> {
        let (edit, remove) = (self.store.clone(), self.store.clone());
        let (edit_entry, remove_entry) = (entry.clone(), entry.clone());
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .py(px(6.0))
            .border_b_1()
            .border_color(theme.border)
            .text_size(px(12.0))
            .child(
                div()
                    .w(px(240.0))
                    .flex_none()
                    .truncate()
                    .font_family(theme.font_mono.clone())
                    .text_color(theme.text_muted)
                    .child(SharedString::from(entry.key.clone())),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_family(theme.font_mono.clone())
                    .child(SharedString::from(entry.value.replace('\n', " ⏎ "))),
            )
            .child(Button::new(("config-edit", ix)).compact().ghost().label("Edit").on_click(move |_, _, cx| {
                edit.update(cx, |this, cx| this.open_config_form(Some(edit_entry.clone()), cx))
            }))
            .child(Button::new(("config-remove", ix)).compact().ghost().label("Remove").on_click(move |_, _, cx| {
                remove.update(cx, |this, cx| this.remove_config(remove_entry.clone(), cx))
            }))
    }
}

impl RenderOnce for ConfigPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let add = self.store.clone();
        let form = |editing: bool| ConfigForm { editing, inputs: self.inputs.clone(), error: self.error.clone(), store: self.store.clone() };
        let rows = self.entries.iter().enumerate().map(|(ix, e)| match &self.edit {
            Some(Edit::Config(editing)) if editing == e => form(true).into_any_element(),
            _ => self.row(ix, e, &theme).into_any_element(),
        });
        let form_open = matches!(self.edit, Some(Edit::AddConfig | Edit::Config(_)));
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.0))
                    .child(div().flex_1().min_w_0().text_size(px(12.0)).text_color(theme.text_faint).child(
                        "The local config of this repository (git config --local). remote.* and user.* keys can also be edited in Remotes and Identity.",
                    ))
                    .child(Button::new("config-add").compact().label("Add entry").on_click(move |_, _, cx| {
                        add.update(cx, |this, cx| this.open_config_form(None, cx))
                    })),
            )
            .children((self.edit == Some(Edit::AddConfig)).then(|| form(false)))
            .children(rows)
            .children(self.notice.map(|n| div().text_size(px(12.0)).text_color(theme.success).child(n)))
            .children((!form_open).then_some(self.error).flatten().map(|e| div().text_size(px(12.0)).text_color(theme.danger).child(e)))
    }
}
