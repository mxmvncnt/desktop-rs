//! Repository settings → Ignored files: the root `.gitignore` and `.git/info/exclude` as text.

use gd_ui::{Button, Theme};
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, prelude::*, px};

use super::state::{IgnoreFile, RepoSettingsInputs, RepoSettingsStore};
use crate::components::text_area;

#[derive(IntoElement)]
pub struct IgnorePage {
    pub inputs: RepoSettingsInputs,
    pub error: Option<String>,
    pub notice: Option<&'static str>,
    pub store: Entity<RepoSettingsStore>,
}

impl RenderOnce for IgnorePage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let section = |file: IgnoreFile, title: &'static str, caption: &'static str, id: &'static str, input| {
            let store = self.store.clone();
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(div().font_weight(gpui::FontWeight::MEDIUM).child(title))
                .child(div().text_size(px(12.0)).text_color(theme.text_faint).child(caption))
                .child(
                    text_area(id, input, "One pattern per line, e.g. target/ or *.log", cx)
                        .w_full()
                        .h(px(100.0))
                        .font_family(theme.font_mono.clone())
                        .text_size(px(12.0))
                        .overflow_y_scroll(),
                )
                .child(
                    div().flex().child(
                        Button::new(id).compact().label("Save").on_click(move |_, _, cx| {
                            store.update(cx, |this, cx| this.save_ignore(file, cx))
                        }),
                    ),
                )
        };
        div()
            .flex()
            .flex_col()
            .gap(px(20.0))
            .child(section(
                IgnoreFile::Gitignore,
                ".gitignore",
                "At the root of the repository. Committed with the project, so everyone shares it.",
                "ignore-gitignore",
                &self.inputs.gitignore,
            ))
            .child(section(
                IgnoreFile::Exclude,
                ".git/info/exclude",
                "Only for this clone: patterns that are not committed.",
                "ignore-exclude",
                &self.inputs.exclude,
            ))
            .children(self.notice.map(|n| div().text_size(px(12.0)).text_color(theme.success).child(n)))
            .children(self.error.map(|e| div().text_size(px(12.0)).text_color(theme.danger).child(e)))
    }
}
