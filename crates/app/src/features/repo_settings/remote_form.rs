//! Add / edit form of one remote: name, fetch URL and an optional separate push URL.

use gd_ui::{Button, Theme};
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

use super::state::{RepoSettingsInputs, RepoSettingsStore};
use crate::components::FieldRow;

#[derive(IntoElement)]
pub struct RemoteForm {
    /// The remote being edited; `None` = adding.
    pub editing: Option<String>,
    pub inputs: RepoSettingsInputs,
    pub error: Option<String>,
    pub store: Entity<RepoSettingsStore>,
}

impl RenderOnce for RemoteForm {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let (save, cancel) = (self.store.clone(), self.store);
        let inp = &self.inputs;
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(12.0))
            .rounded(px(gd_ui::theme::CONTROL_RADIUS + 2.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.ink(0.03))
            .child(FieldRow::new(
                "Name",
                "remote-name",
                &inp.remote_name,
                "origin",
            ))
            .child(FieldRow::new(
                "Fetch URL",
                "remote-url",
                &inp.remote_url,
                "https://github.com/owner/repo.git",
            ))
            .child(FieldRow::new(
                "Push URL (optional, defaults to the fetch URL)",
                "remote-push-url",
                &inp.remote_push_url,
                "",
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        Button::new("remote-save")
                            .primary()
                            .compact()
                            .label(if self.editing.is_some() {
                                "Save"
                            } else {
                                "Add"
                            })
                            .on_click(move |_, _, cx| {
                                save.update(cx, |this, cx| this.save_remote(cx))
                            }),
                    )
                    .child(
                        Button::new("remote-cancel")
                            .ghost()
                            .compact()
                            .label("Cancel")
                            .on_click(move |_, _, cx| {
                                cancel.update(cx, |this, cx| this.close_edit(cx))
                            }),
                    )
                    .children(
                        self.error
                            .map(|e| div().text_size(px(12.0)).text_color(theme.danger).child(e)),
                    ),
            )
    }
}
