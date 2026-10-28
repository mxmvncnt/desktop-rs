//! Scrollable list of branches to start a new branch from.

use gd_git::BranchKind;
use gd_ui::{Icon, Theme, icon};
use gpui::{
    App, Entity, IntoElement, ParentElement, RenderOnce, Styled,
    Window, div, px,
};

use super::new_branch_base::NewBranchBase;
use crate::components::{BranchRow, list};
use super::store::PopoverStore;

const ROW_H: f32 = 28.0;
const VISIBLE_ROWS: f32 = 6.0;

#[derive(IntoElement)]
pub struct NewBranchBaseList {
    branches: Vec<gd_git::Branch>,
    current: String,
    selected: String,
    pickers: Entity<PopoverStore>,
}

impl NewBranchBaseList {
    pub fn new(base: &NewBranchBase) -> Self {
        Self {
            branches: base.branches.clone(),
            current: base.current.clone(),
            selected: base.effective().to_string(),
            pickers: base.pickers.clone(),
        }
    }
}

impl RenderOnce for NewBranchBaseList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let n = self.branches.len();
        let height = (n as f32).min(VISIBLE_ROWS) * ROW_H + 8.0;
        let Self { branches, current, selected, pickers } = self;
        div()
            .h(px(height))
            .rounded(px(6.0))
            .border_1()
            .border_color(theme.border)
            .child(
                list("new-branch-base-list", n, move |range, _w, cx| {
                    range
                        .filter_map(|i| Some((i, branches.get(i)?.clone())))
                        .map(|(i, b)| base_row(i, b, &current, &selected, &pickers, cx))
                        .collect()
                })
                .size_full(),
            )
    }
}

/// Hot row (virtualized): a plain fn, not a component.
fn base_row(
    i: usize,
    b: gd_git::Branch,
    current: &str,
    selected: &str,
    pickers: &Entity<PopoverStore>,
    cx: &mut App,
) -> gpui::AnyElement {
    let pickers = pickers.clone();
    let is_selected = b.name == selected;
    let pick = (b.name != current).then(|| b.name.clone());
    let glyph = if b.kind == BranchKind::Remote { Icon::Cloud } else { Icon::GitBranch };
    BranchRow::new("new-branch-base-row", i, b.name.clone(), move |_, cx| {
        let pick = pick.clone();
        pickers.update(cx, |p, cx| p.set_base(pick, cx))
    })
    .icon(glyph)
    .height(ROW_H)
    .bright(is_selected)
    .children(is_selected.then(|| icon(Icon::Check).size(px(14.0)).text_color(Theme::of(cx).text)))
    .into_any_element()
}
