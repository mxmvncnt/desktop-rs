//! Segmented tab bar.

use gpui::{App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, millis, prelude::*, px};

use super::badge;
use crate::theme::{CONTROL_RADIUS, Theme, ink};

/// Segmented tab bar ("Changes" | "History"). Selected tab gets a raised
/// plate; the plate color transitions so switching feels like it slides.
#[derive(IntoElement)]
pub struct Tabs {
    id: ElementId,
    items: Vec<(SharedString, Option<SharedString>)>,
    selected: usize,
    on_select: Option<std::rc::Rc<dyn Fn(usize, &mut Window, &mut App)>>,
}

impl Tabs {
    pub fn new(id: impl Into<ElementId>, selected: usize) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            selected,
            on_select: None,
        }
    }
    pub fn tab(mut self, label: impl Into<SharedString>, badge: Option<SharedString>) -> Self {
        self.items.push((label.into(), badge));
        self
    }
    pub fn on_select(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(std::rc::Rc::new(f));
        self
    }
}

impl RenderOnce for Tabs {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        div()
            .id(self.id)
            .flex()
            .gap(px(2.0))
            .p(px(3.0))
            .rounded(px(CONTROL_RADIUS + 3.0))
            .bg(ink(0.03))
            .children(self.items.into_iter().enumerate().map(|(ix, (label, count))| {
                let selected = ix == self.selected;
                let on_select = self.on_select.clone();
                div()
                    .id(ix)
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.0))
                    .h(px(26.0))
                    .rounded(px(CONTROL_RADIUS))
                    .text_size(px(13.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .cursor_pointer()
                    .bg(if selected { ink(0.09) } else { ink(0.0) })
                    .text_color(if selected { theme.text } else { theme.text_faint })
                    .transitions(|t| t.bg(millis(180)).text_color(millis(180)))
                    .hover_style({
                        let c = theme.text;
                        move |s| s.text_color(c)
                    })
                    .child(label)
                    .when_some(count, |d, b| d.child(badge(b, cx)))
                    .when_some(on_select, |d, f| d.on_click(move |_, w, cx| f(ix, w, cx)))
            }))
    }
}
