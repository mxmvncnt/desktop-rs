//! "Based on" field of the new-branch popover: a combo box with the start
//! point, empty for the current branch, that lists the branches to start from.

use gd_git::Branch;
use gd_ui::Theme;
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

use super::store::PopoverStore;
use crate::components::ComboBox;

#[derive(IntoElement)]
pub struct NewBranchBase {
    /// Checked-out branch (the default start point).
    pub current: String,
    /// Explicit start point, `None` = current branch.
    pub chosen: Option<String>,
    pub branches: Vec<Branch>,
    pub combo: Entity<ComboBox>,
    pub pickers: Entity<PopoverStore>,
}

impl NewBranchBase {
    /// Name of the branch the new one will start from.
    pub fn effective(&self) -> &str {
        self.chosen.as_deref().unwrap_or(&self.current)
    }
}

impl RenderOnce for NewBranchBase {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.text_muted)
                    .child("Based on"),
            )
            .child(self.combo)
    }
}
