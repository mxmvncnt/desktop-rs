//! Backdrop + positioned card hosting whichever popover is open.

use gd_ui::theme::TOOLBAR_HEIGHT;
use gpui::{AnyElement, App, Context, Entity, IntoElement, RenderOnce, Window, div, point, px};

use crate::workspace::{Popover, Workspace};

#[derive(IntoElement)]
pub struct PopoverLayer {
    ws: Entity<Workspace>,
    popover: Popover,
    generation: usize,
    content: AnyElement,
}

impl PopoverLayer {
    pub fn open(ws: &mut Workspace, popover: Popover, cx: &mut Context<Workspace>) -> Self {
        let content = match popover {
            Popover::Repositories => ws.repo_picker(cx).into_any_element(),
            Popover::Branches => match ws.branch_picker(cx) {
                Some(picker) => picker.into_any_element(),
                None => div().into_any_element(),
            },
            Popover::NewBranch => ws.new_branch_form(cx).into_any_element(),
            Popover::Clone => ws.clone_form(cx).into_any_element(),
        };
        Self { ws: cx.entity(), popover, generation: ws.overlay_gen, content }
    }
}

impl RenderOnce for PopoverLayer {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (left, width) = match self.popover {
            Popover::Repositories | Popover::Clone => (8.0, 380.0),
            Popover::Branches | Popover::NewBranch => (296.0, 380.0),
        };
        let ws = self.ws;
        gd_ui::Popover::new(("popover", self.generation), point(px(left), px(TOOLBAR_HEIGHT - 2.0)), width, self.content)
            .scrim(0.18)
            .on_dismiss(move |ev, window, cx| {
                ws.update(cx, |this, cx| {
                    let dismissed = this.popover().map(|p| (p, ev.position));
                    this.pickers.update(cx, |p, _| p.dismissed = dismissed);
                    this.close_overlay(window, cx)
                })
            })
    }
}
