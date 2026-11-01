//! The dropdown of the branch filter: "All branches", the current branch and the others, to pick instead of typing.

use std::collections::HashSet;
use std::sync::Arc;

use gd_git::{Branch, BranchKind};
use gd_ui::overlay_card;
use gpui::{
    App, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce, SharedString, Styled, Window, deferred, div, px,
    uniform_list,
};
use gpui_elements::editable_text::EditableTextState;

use super::state::CiStore;
use crate::components::BranchRow;

const ROW_HEIGHT: f32 = 28.0;
const MAX_ROWS: usize = 10;
const WIDTH: f32 = 260.0;

/// The names a pipeline can be filtered on: the local branches, then the remote ones that are not local (without their remote), the current branch first.
pub fn branch_names(branches: &[Branch], current: Option<&str>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut names: Vec<String> = branches
        .iter()
        .filter_map(|b| match b.kind {
            BranchKind::Local => Some(b.name.clone()),
            BranchKind::Remote => b.name.split_once('/').map(|(_, name)| name.to_string()),
        })
        .filter(|name| name != "HEAD" && seen.insert(name.clone()))
        .collect();
    if let Some(ix) = names.iter().position(|n| Some(n.as_str()) == current) {
        names[..=ix].rotate_right(1);
    }
    names
}

#[derive(IntoElement)]
pub struct BranchMenu {
    pub store: Entity<CiStore>,
    /// The branch field: picking a branch writes it there, and that is what reloads the list.
    pub input: Entity<EditableTextState>,
    /// What the field holds now.
    pub typed: String,
    pub current: Option<String>,
    pub branches: Vec<String>,
}

impl RenderOnce for BranchMenu {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // (label, value): "All branches" clears the filter.
        let entries: Arc<Vec<(SharedString, String)>> = Arc::new(
            std::iter::once(("All branches".into(), String::new()))
                .chain(self.branches.iter().map(|name| {
                    let label = if self.current.as_deref() == Some(name) { format!("{name} (current)") } else { name.clone() };
                    (label.into(), name.clone())
                }))
                .collect(),
        );
        let height = entries.len().min(MAX_ROWS) as f32 * ROW_HEIGHT;
        let (store, input, typed, close) = (self.store.clone(), self.input, self.typed, self.store);
        let rows = {
            let entries = entries.clone();
            uniform_list("ci-branch-menu", entries.len(), move |range, _, _| {
                range
                    .map(|ix| {
                        let (label, value) = &entries[ix];
                        let (store, input, picked) = (store.clone(), input.clone(), value.clone());
                        BranchRow::new("ci-branch-pick", ix, label.clone(), move |_, cx| {
                            input.update(cx, |s, cx| s.emplace(&picked, cx));
                            store.update(cx, |s, cx| {
                                s.branch_menu = false;
                                cx.notify()
                            });
                        })
                        .height(ROW_HEIGHT)
                        .selected(*value == typed)
                    })
                    .collect()
            })
            .h(px(height))
        };
        // The backdrop catches the click that dismisses the menu wherever it lands (see `accounts::AccountForm::provider_select`).
        deferred(
            div()
                .absolute()
                .top_0()
                .left_0()
                .child(div().absolute().top(px(-3000.0)).left(px(-3000.0)).size(px(6000.0)).occlude().on_mouse_down(
                    MouseButton::Left,
                    move |_, _, cx| {
                        close.update(cx, |s, cx| {
                            s.branch_menu = false;
                            cx.notify()
                        })
                    },
                ))
                .child(div().absolute().top(px(36.0)).left_0().w(px(WIDTH)).child(overlay_card(cx).occlude().p(px(4.0)).child(rows))),
        )
        .priority(2)
    }
}

#[cfg(test)]
#[path = "tests/branch_menu.test.rs"]
mod tests;
