//! Checkbox with checked / unchecked / mixed states.

use gpui::{App, ClickEvent, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, div, millis, prelude::*, px};

use super::{ClickHandler, icon};
use crate::assets::Icon;
use crate::motion::HOVER_MS;
use crate::theme::{Theme, ink};

/// The checkbox used in the changes list.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    state: CheckState,
    on_click: Option<ClickHandler>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CheckState {
    Checked,
    Unchecked,
    Mixed,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>, state: CheckState) -> Self {
        Self {
            id: id.into(),
            state,
            on_click: None,
        }
    }
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let on = self.state != CheckState::Unchecked;
        let mut el = div()
            .id(self.id)
            .size(px(15.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.0))
            .border_1()
            .border_color(if on { theme.solid } else { theme.border_strong })
            .bg(if on { theme.solid } else { ink(0.0) })
            .transitions(|t| t.bg(millis(HOVER_MS)).border_color(millis(HOVER_MS)))
            .hover_style({
                let c = theme.text_muted;
                move |s| s.border_color(c)
            })
            .cursor_pointer()
            .map(|d| match self.state {
                CheckState::Checked => {
                    d.child(icon(Icon::Check).size(px(12.0)).text_color(theme.on_solid))
                }
                CheckState::Mixed => d.child(
                    div()
                        .w(px(7.0))
                        .h(px(1.5))
                        .rounded(px(1.0))
                        .bg(theme.on_solid),
                ),
                CheckState::Unchecked => d,
            });
        if let Some(f) = self.on_click {
            el = el.on_click(move |e, w, cx| {
                cx.stop_propagation();
                f(e, w, cx)
            });
        }
        el
    }
}
