//! Drag handle for resizable panes. Dragging emits the `P` payload (handle it
//! with `on_drag_move::<P>` on an ancestor); double-click calls `on_reset`.

use gpui::{
    AppContext as _, ClickEvent, ColorExt as _, Context, CursorStyle, ElementId, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Render, RenderOnce, StatefulInteractiveElement, Styled, Window, div, px,
};

use std::rc::Rc;

use super::Callback;
use crate::theme::Theme;

/// The (invisible) view GPUI shows under the cursor while dragging.
struct NoGhost;

impl Render for NoGhost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

enum Edge {
    /// Thin strip on the right edge of a `relative` parent.
    Right,
    /// Full-width row with a centered grip pill, below its content.
    Bottom,
}

#[derive(IntoElement)]
pub struct ResizeHandle<P: 'static> {
    id: ElementId,
    edge: Edge,
    payload: P,
    on_reset: Option<Callback>,
}

impl<P: 'static> ResizeHandle<P> {
    pub fn right_edge(id: impl Into<ElementId>, payload: P) -> Self {
        Self { id: id.into(), edge: Edge::Right, payload, on_reset: None }
    }

    pub fn bottom_edge(id: impl Into<ElementId>, payload: P) -> Self {
        Self { id: id.into(), edge: Edge::Bottom, payload, on_reset: None }
    }

    /// Called on double-click.
    pub fn on_reset(mut self, f: impl Fn(&mut Window, &mut gpui::App) + 'static) -> Self {
        self.on_reset = Some(Rc::new(f));
        self
    }
}

impl<P: 'static> RenderOnce for ResizeHandle<P> {
    fn render(self, _window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let (accent, line) = (theme.accent, theme.border);
        let payload = self.payload;
        let handle = div()
            .id(self.id.clone())
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_drag(payload, |_, _, _, cx| cx.new(|_| NoGhost))
            .on_click(move |e: &ClickEvent, w, cx| {
                if let (2, Some(f)) = (e.click_count(), &self.on_reset) {
                    f(w, cx);
                }
            });
        match self.edge {
            Edge::Right => handle
                .absolute()
                .top_0()
                .bottom_0()
                .right(px(-3.0))
                .w(px(6.0))
                .cursor(CursorStyle::ResizeLeftRight)
                .hover_style(move |s| s.bg(accent.opacity(0.5)))
                .into_any_element(),
            Edge::Bottom => {
                let group = format!("{}-group", self.id);
                handle
                    .h(px(8.0))
                    .w_full()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor(CursorStyle::ResizeUpDown)
                    .group(group.clone())
                    .child(
                        div()
                            .w(px(36.0))
                            .h(px(3.0))
                            .rounded(px(2.0))
                            .bg(line)
                            .group_hover_style(group, move |s| s.bg(accent.opacity(0.6))),
                    )
                    .into_any_element()
            }
        }
    }
}
