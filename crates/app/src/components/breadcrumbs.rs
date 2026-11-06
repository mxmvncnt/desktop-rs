//! The breadcrumb bar of a forge page's inner screens: back arrow, then the trail (`Pipelines › #12 › build`).
//! A fixed height, so the bar stays at the same place on every screen.

use gd_ui::{Callback, Icon, Theme, icon, icon_button};
use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, px,
};

/// One step of the trail: a link, or the current screen (`on_click: None`).
pub struct Crumb {
    pub label: String,
    pub on_click: Option<Callback>,
}

impl Crumb {
    pub fn link(
        label: impl Into<String>,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            on_click: Some(std::rc::Rc::new(on_click)),
        }
    }

    pub fn here(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on_click: None,
        }
    }
}

#[derive(IntoElement)]
pub struct Breadcrumbs {
    pub id: &'static str,
    pub on_back: Callback,
    pub crumbs: Vec<Crumb>,
}

impl RenderOnce for Breadcrumbs {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let on_back = self.on_back;
        let crumbs = self.crumbs.into_iter().enumerate().map(|(ix, crumb)| {
            let label = match crumb.on_click {
                Some(on_click) => div()
                    .id(ElementId::NamedInteger(self.id.into(), ix as u64))
                    .cursor_pointer()
                    .hover_style(|s| s.underline())
                    .child(crumb.label)
                    .on_click(move |_, w, cx| on_click(w, cx))
                    .into_any_element(),
                None => div()
                    .text_color(theme.text)
                    .child(crumb.label)
                    .into_any_element(),
            };
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .min_w_0()
                .children((ix > 0).then(|| {
                    icon(Icon::ChevronRight)
                        .size(px(12.0))
                        .text_color(theme.text_faint)
                }))
                .child(label)
        });
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(6.0))
            .h(px(56.0))
            .px(px(24.0))
            .text_color(theme.text_muted)
            .child(
                icon_button(
                    ElementId::Name(format!("{}-back", self.id).into()),
                    Icon::ArrowLeft,
                )
                .on_click(move |_, w, cx| on_back(w, cx)),
            )
            .children(crumbs)
    }
}
