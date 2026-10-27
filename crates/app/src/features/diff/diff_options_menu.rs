//! The diff options popover: closes on backdrop click/Esc.

use gd_ui::Popover;
use gpui::{App, Entity, IntoElement, ParentElement, Pixels, Point, RenderOnce, Styled, Window, div, px};

use crate::components::DiffDisplayOptions;
use crate::workspace::Workspace;

const MENU_W: f32 = 264.0;

#[derive(IntoElement)]
pub struct DiffOptionsMenu {
    pos: Point<Pixels>,
    /// Popover open counter: restarts the entrance animation per open.
    generation: usize,
    options: DiffDisplayOptions,
    ws: Entity<Workspace>,
}

impl DiffOptionsMenu {
    pub fn new(ws: &Workspace, pos: Point<Pixels>, entity: Entity<Workspace>) -> Self {
        Self {
            pos,
            generation: ws.overlay_gen,
            options: DiffDisplayOptions::new(&ws.settings, true, entity.clone()),
            ws: entity,
        }
    }
}

impl RenderOnce for DiffOptionsMenu {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let ws = self.ws;
        let pos = origin(self.pos, window);
        Popover::new(("diff-options-in", self.generation), pos, MENU_W, div().p(px(6.0)).child(self.options))
            .on_dismiss(move |_, w, cx| ws.update(cx, |this, cx| this.close_overlay(w, cx)))
    }
}

/// Top-left of the card: under the gear button, kept inside the window.
fn origin(pos: Point<Pixels>, window: &Window) -> Point<Pixels> {
    let inset = window.client_inset().unwrap_or(px(0.0));
    let view = window.viewport_size();
    let x = (pos.x - inset - px(MENU_W - 24.0)).min(view.width - inset * 2.0 - px(MENU_W + 8.0)).max(px(8.0));
    Point { x, y: pos.y - inset + px(18.0) }
}
