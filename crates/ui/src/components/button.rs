//! Buttons: primary plate, secondary pill, ghost and danger, plus the
//! icon-only ghost button.

use gpui::{App, ClickEvent, ElementId, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, millis, prelude::*, px};

use super::{ClickHandler, icon};
use crate::assets::Icon;
use crate::motion::HOVER_MS;
use crate::theme::{CONTROL_RADIUS, Theme};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    /// Near-white solid plate, dark label. The one primary action per view.
    Primary,
    /// Translucent raised pill.
    Secondary,
    /// No plate until hovered.
    Ghost,
    /// Red plate for destructive confirmations.
    Danger,
}

#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: Option<SharedString>,
    icon: Option<Icon>,
    style: ButtonStyle,
    disabled: bool,
    full_width: bool,
    compact: bool,
    on_click: Option<ClickHandler>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            icon: None,
            style: ButtonStyle::Secondary,
            disabled: false,
            full_width: false,
            compact: false,
            on_click: None,
        }
    }
    pub fn label(mut self, l: impl Into<SharedString>) -> Self {
        self.label = Some(l.into());
        self
    }
    pub fn icon(mut self, i: Icon) -> Self {
        self.icon = Some(i);
        self
    }
    pub fn style(mut self, s: ButtonStyle) -> Self {
        self.style = s;
        self
    }
    pub fn primary(self) -> Self {
        self.style(ButtonStyle::Primary)
    }
    pub fn ghost(self) -> Self {
        self.style(ButtonStyle::Ghost)
    }
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let (bg, hover, fg): (Hsla, Hsla, Hsla) = match self.style {
            ButtonStyle::Primary => (theme.solid, theme.solid_hover, theme.on_solid),
            ButtonStyle::Secondary => (theme.ink(0.06), theme.ink(0.10), theme.text),
            ButtonStyle::Ghost => (theme.ink(0.0), theme.element_hover, theme.text_muted),
            ButtonStyle::Danger => (theme.danger_strong, theme.danger, theme.on_accent),
        };
        let hover_fg = if self.style == ButtonStyle::Ghost {
            theme.text
        } else {
            fg
        };
        let mut el = div()
            .id(self.id)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .h(px(if self.compact { 26.0 } else { 30.0 }))
            .px(px(if self.label.is_some() { 12.0 } else { 7.0 }))
            .rounded(px(CONTROL_RADIUS))
            .text_size(px(13.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .bg(bg)
            .text_color(fg)
            .when(self.full_width, |d| d.w_full())
            .when_some(self.icon, |d, i| d.child(icon(i).size(px(15.0)).text_color(fg)))
            .when_some(self.label, |d, l| d.child(l));
        if self.disabled {
            el = el.opacity(0.4);
        } else {
            el = el
                .cursor_pointer()
                .transitions(|t| t.bg(millis(HOVER_MS)).text_color(millis(HOVER_MS)))
                .hover_style(move |s| s.bg(hover).text_color(hover_fg));
            if let Some(f) = self.on_click {
                el = el.on_click(f);
            }
        }
        el
    }
}

/// Square icon-only ghost button (toolbar/header affordances).
pub fn icon_button(id: impl Into<ElementId>, i: Icon) -> Button {
    Button::new(id).icon(i).ghost()
}
