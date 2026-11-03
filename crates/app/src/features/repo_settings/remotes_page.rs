//! Repository settings → Remotes: the list with fetch / push URLs, add, edit and remove.

use gd_git::Remote;
use gd_ui::{Button, ButtonStyle, Theme};
use gpui::{
    App, Entity, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px,
};

use super::remote_form::RemoteForm;
use super::state::{Edit, RepoSettingsInputs, RepoSettingsStore};

#[derive(IntoElement)]
pub struct RemotesPage {
    pub remotes: Vec<Remote>,
    pub edit: Option<Edit>,
    pub error: Option<String>,
    pub inputs: RepoSettingsInputs,
    pub store: Entity<RepoSettingsStore>,
}

impl RemotesPage {
    fn row(&self, remote: &Remote, theme: &Theme) -> impl IntoElement + use<> {
        let name = remote.name.clone();
        let (edit, ask) = (self.store.clone(), self.store.clone());
        let (edit_name, ask_name) = (name.clone(), name.clone());
        let url = |label: &'static str, url: &str| {
            div()
                .flex()
                .gap(px(8.0))
                .text_size(px(12.0))
                .child(div().w(px(36.0)).text_color(theme.text_faint).child(label))
                .child(
                    div()
                        .min_w_0()
                        .font_family(theme.font_mono.clone())
                        .child(SharedString::from(url.to_string())),
                )
        };
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .py(px(10.0))
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(3.0))
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(name.clone()),
                    )
                    .child(url("fetch", &remote.fetch_url))
                    .child(url("push", &remote.push_url)),
            )
            .child(
                Button::new(SharedString::from(format!("remote-edit-{name}")))
                    .compact()
                    .ghost()
                    .label("Edit")
                    .on_click(move |_, _, cx| {
                        edit.update(cx, |this, cx| {
                            this.open_remote_form(Some(edit_name.clone()), cx)
                        })
                    }),
            )
            .child(
                Button::new(SharedString::from(format!("remote-remove-{name}")))
                    .compact()
                    .ghost()
                    .label("Remove")
                    .on_click(move |_, _, cx| {
                        ask.update(cx, |this, cx| {
                            this.confirm_remove_remote(ask_name.clone(), cx)
                        })
                    }),
            )
    }

    fn confirm(&self, name: &str, theme: &Theme) -> impl IntoElement + use<> {
        let (remove, cancel) = (self.store.clone(), self.store.clone());
        let target = name.to_string();
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .py(px(10.0))
            .border_b_1()
            .border_color(theme.border)
            .child(div().flex_1().child(format!(
                "Remove the remote {name}? Its remote-tracking branches are deleted too."
            )))
            .child(
                Button::new("remote-remove-confirm")
                    .compact()
                    .style(ButtonStyle::Danger)
                    .label("Remove")
                    .on_click(move |_, _, cx| {
                        remove.update(cx, |this, cx| this.remove_remote(target.clone(), cx))
                    }),
            )
            .child(
                Button::new("remote-remove-cancel")
                    .compact()
                    .ghost()
                    .label("Cancel")
                    .on_click(move |_, _, cx| cancel.update(cx, |this, cx| this.close_edit(cx))),
            )
    }
}

impl RenderOnce for RemotesPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let add = self.store.clone();
        let form = |editing: Option<String>| RemoteForm {
            editing,
            inputs: self.inputs.clone(),
            error: self.error.clone(),
            store: self.store.clone(),
        };
        let rows = self.remotes.iter().map(|r| match &self.edit {
            Some(Edit::Remote(n)) if *n == r.name => form(Some(r.name.clone())).into_any_element(),
            Some(Edit::RemoveRemote(n)) if *n == r.name => {
                self.confirm(&r.name, &theme).into_any_element()
            }
            _ => self.row(r, &theme).into_any_element(),
        });
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.text_faint)
                            .child("git remote: where this repository fetches from and pushes to."),
                    )
                    .child(
                        Button::new("remote-add")
                            .compact()
                            .label("Add remote")
                            .on_click(move |_, _, cx| {
                                add.update(cx, |this, cx| this.open_remote_form(None, cx))
                            }),
                    ),
            )
            .children((self.edit == Some(Edit::AddRemote)).then(|| form(None)))
            .children(rows)
            .children(self.remotes.is_empty().then(|| {
                div()
                    .py(px(12.0))
                    .text_color(theme.text_muted)
                    .child("This repository has no remote.")
            }))
            .children(match (&self.edit, self.error) {
                (Some(Edit::RemoveRemote(_)), Some(e)) | (None, Some(e)) => {
                    Some(div().text_size(px(12.0)).text_color(theme.danger).child(e))
                }
                _ => None,
            })
    }
}
