//! Header above the diff: file path (with the old path for renames) and the
//! diff options button.

use gd_git::FileChange;
use gd_ui::{Icon, Theme, icon};
use gpui::{App, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

use crate::components::{drag_selection, selectable_line};
use crate::features::diff::DiffOptionsButton;
use crate::features::history::CommitSelection;
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct FileHeader {
    title: String,
    sel: CommitSelection,
    ws: Entity<Workspace>,
}

impl FileHeader {
    pub fn new(change: &FileChange, sel: CommitSelection, ws: Entity<Workspace>) -> Self {
        let title = match &change.old_path {
            Some(old) => format!("{old} → {}", change.path),
            None => change.path.clone(),
        };
        Self { title, sel, ws }
    }
}

impl RenderOnce for FileHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let texts = [self.title, String::new()];
        let sel = self.sel.on(&texts);
        let (title, layout) = selectable_line(&self.ws, &texts, 0, sel, theme.text_muted, FontWeight::NORMAL, &theme);
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.0))
            .h(px(40.0))
            .px(px(16.0))
            .border_b_1()
            .border_color(theme.border)
            .on_mouse_move(drag_selection(self.ws.clone(), vec![layout]))
            .child(icon(Icon::Document).size(px(14.0)).text_color(theme.text_faint))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.5))
                    .text_color(theme.text_muted)
                    .child(title),
            )
            .child(DiffOptionsButton { ws: self.ws })
    }
}
