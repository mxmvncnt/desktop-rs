//! Header above the diff: file path (with the old path for renames) and the
//! diff options button.

use gd_git::FileChange;
use gd_ui::{Icon, Theme, icon};
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

use crate::features::diff::DiffOptionsButton;
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct FileHeader {
    title: String,
    ws: Entity<Workspace>,
}

impl FileHeader {
    pub fn new(change: &FileChange, ws: Entity<Workspace>) -> Self {
        let title = match &change.old_path {
            Some(old) => format!("{old} → {}", change.path),
            None => change.path.clone(),
        };
        Self { title, ws }
    }
}

impl RenderOnce for FileHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.0))
            .h(px(40.0))
            .px(px(16.0))
            .border_b_1()
            .border_color(theme.border)
            .child(icon(Icon::Document).size(px(14.0)).text_color(theme.text_faint))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.5))
                    .text_color(theme.text_muted)
                    .child(self.title),
            )
            .child(DiffOptionsButton { ws: self.ws })
    }
}
