//! Full-window popover layer: click-away backdrop plus a card at a window
//! position, with the menu entrance animation. Stateless: the owner renders it
//! only while open and positions it (clamping to the window is up to the caller).

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels,
    Point, RenderOnce, Styled, Window, deferred, div, px,
};

use super::overlay_card;
use crate::motion;
use crate::theme::scrim;

#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    pos: Point<Pixels>,
    width: f32,
    content: AnyElement,
    scrim: Option<f32>,
    on_dismiss: Option<Rc<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>>,
}

impl Popover {
    /// `id` must change per open so the entrance animation restarts.
    pub fn new(id: impl Into<ElementId>, pos: Point<Pixels>, width: f32, content: impl IntoElement) -> Self {
        Self { id: id.into(), pos, width, content: content.into_any_element(), scrim: None, on_dismiss: None }
    }

    /// Dim the window behind the card with this opacity.
    pub fn scrim(mut self, opacity: f32) -> Self {
        self.scrim = Some(opacity);
        self
    }

    /// Called for a left or right click on the backdrop.
    pub fn on_dismiss(mut self, f: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Popover {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let instant = motion::handoff(&self.id, cx);
        let mut backdrop = div().id("popover-backdrop").occlude().absolute().inset_0();
        if let Some(opacity) = self.scrim {
            backdrop = backdrop.bg(scrim(opacity));
        }
        if let Some(f) = self.on_dismiss {
            let g = f.clone();
            backdrop = backdrop
                .on_mouse_down(MouseButton::Left, move |e, w, cx| f(e, w, cx))
                .on_mouse_down(MouseButton::Right, move |e, w, cx| g(e, w, cx));
        }
        deferred(div().absolute().inset_0().child(backdrop).child(
            div().absolute().left(self.pos.x).top(self.pos.y).w(px(self.width)).child(motion::menu_in(
                self.id,
                overlay_card(cx).id("popover-card").occlude().child(self.content),
                instant,
            )),
        ))
    }
}
