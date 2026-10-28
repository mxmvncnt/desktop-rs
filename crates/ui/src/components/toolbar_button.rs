//! Toolbar dropdown button (caption over title, optional chevron and busy pulse).

use gpui::{AnyElement, App, ClickEvent, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, prelude::*, px};
use gpui::AnimationExt as _;

use super::{ClickHandler, icon, pressable};
use crate::assets::Icon;
use crate::theme::{CONTROL_RADIUS, Theme};

/// Toolbar dropdown in Desktop's layout ("Current repository ▾"), drawn
/// zeron-style: small muted caption over a medium title, ghost hover wash.
#[derive(IntoElement)]
pub struct ToolbarButton {
    id: ElementId,
    icon: Icon,
    caption: SharedString,
    title: SharedString,
    chevron: bool,
    open: bool,
    busy: bool,
    width: Option<f32>,
    on_click: Option<ClickHandler>,
}

impl ToolbarButton {
    pub fn new(
        id: impl Into<ElementId>,
        icon: Icon,
        caption: impl Into<SharedString>,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            caption: caption.into(),
            title: title.into(),
            chevron: false,
            open: false,
            busy: false,
            width: None,
            on_click: None,
        }
    }
    pub fn chevron(mut self) -> Self {
        self.chevron = true;
        self
    }
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
}

impl RenderOnce for ToolbarButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let glyph = icon(self.icon).text_color(theme.text_muted);
        let icon_el: AnyElement = if self.busy {
            glyph
                .with_animation(
                    "busy",
                    gpui::Animation::new(std::time::Duration::from_millis(900))
                        .repeat()
                        .with_easing(gpui::pulsating_between(0.35, 1.0)),
                    |d, t| d.opacity(t),
                )
                .into_any_element()
        } else {
            glyph.into_any_element()
        };
        let mut el = pressable(self.id, self.open, cx)
            .flex()
            .items_center()
            .gap(px(10.0))
            .h_full()
            .px(px(14.0))
            .rounded(px(CONTROL_RADIUS + 2.0))
            .when_some(self.width, |d, w| d.w(px(w)))
            .when(self.width.is_none(), |d| d.min_w(px(180.0)))
            .child(icon_el)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.text_faint)
                            .line_height(px(14.0))
                            .child(self.caption),
                    )
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .line_height(px(17.0))
                            .truncate()
                            .child(self.title),
                    ),
            )
            .when(self.chevron, |d| {
                d.child(
                    icon(if self.open {
                        Icon::ChevronUp
                    } else {
                        Icon::ChevronDown
                    })
                    .size(px(14.0))
                    .text_color(theme.text_faint),
                )
            });
        if let Some(f) = self.on_click {
            el = el.on_click(f);
        }
        el
    }
}
