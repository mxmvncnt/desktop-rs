//! Warning under "Based on" when the local base branch is behind its upstream
//! (as of the last fetch), with a link to start from the upstream instead.

use gd_git::BranchKind;
use gd_ui::{Icon, Theme, icon};
use gpui::{
    App, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use super::new_branch_base::NewBranchBase;
use super::store::PopoverStore;
use crate::utils::text::plural;

#[derive(IntoElement)]
pub struct StaleBaseNotice {
    base: String,
    upstream: String,
    behind: u32,
    pickers: Entity<PopoverStore>,
}

impl StaleBaseNotice {
    pub fn new(base: &NewBranchBase) -> Option<Self> {
        let name = base.effective();
        let b = base
            .branches
            .iter()
            .find(|b| b.kind == BranchKind::Local && b.name == name)?;
        let behind = b.ahead_behind?.behind;
        if behind == 0 {
            return None;
        }
        Some(Self {
            base: name.to_string(),
            upstream: b.upstream.clone()?,
            behind,
            pickers: base.pickers.clone(),
        })
    }
}

impl RenderOnce for StaleBaseNotice {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let n = self.behind;
        let message = format!(
            "{} is {n} commit{} behind {}.",
            self.base,
            plural(n),
            self.upstream
        );
        let (pickers, upstream) = (self.pickers, self.upstream.clone());
        div()
            .flex()
            .items_start()
            .gap(px(8.0))
            .p(px(10.0))
            .rounded(px(gd_ui::theme::CONTROL_RADIUS + 1.0))
            .bg(theme.ink(0.04))
            .border_1()
            .border_color(theme.border)
            .child(icon(Icon::Warning).size(px(14.0)).text_color(theme.warning))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .text_size(px(12.0))
                    .text_color(theme.text_muted)
                    .child(message)
                    .child(
                        div()
                            .id("stale-base-use-upstream")
                            .cursor_pointer()
                            .text_color(theme.text)
                            .underline()
                            .child(format!("Start from {} instead", self.upstream))
                            .on_click(move |_, _, cx| {
                                let upstream = upstream.clone();
                                pickers.update(cx, |p, cx| p.set_base(Some(upstream), cx))
                            }),
                    ),
            )
    }
}
