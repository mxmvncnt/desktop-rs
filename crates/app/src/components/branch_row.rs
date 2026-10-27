//! One branch: icon, name and trailing children (time, check, tags).

use std::rc::Rc;

use gd_ui::{Callback, Icon, Theme, icon, pressable};
use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::utils::text::relative_time;

type RightClick = Box<dyn Fn(Point<Pixels>, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct BranchRow {
    /// Element id namespace, so lists on screen together never collide.
    id: &'static str,
    ix: usize,
    name: SharedString,
    icon: Icon,
    height: f32,
    selected: bool,
    bright: bool,
    trailing: Vec<AnyElement>,
    on_click: Callback,
    on_right_click: Option<RightClick>,
}

impl BranchRow {
    pub fn new(
        id: &'static str,
        ix: usize,
        name: impl Into<SharedString>,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id,
            ix,
            name: name.into(),
            icon: Icon::GitBranch,
            height: 30.0,
            selected: false,
            bright: false,
            trailing: Vec::new(),
            on_click: Rc::new(on_click),
            on_right_click: None,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Highlights the row (keyboard cursor).
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Brighter name and icon (current or chosen branch).
    pub fn bright(mut self, bright: bool) -> Self {
        self.bright = bright;
        self
    }

    pub fn on_right_click(mut self, f: impl Fn(Point<Pixels>, &mut Window, &mut App) + 'static) -> Self {
        self.on_right_click = Some(Box::new(f));
        self
    }

    /// The faint relative tip time shown at the end of a row.
    pub fn tip_time(tip_time: i64, cx: &App) -> impl IntoElement + use<> {
        let faint = Theme::of(cx).text_faint;
        div().flex_none().text_size(px(11.0)).text_color(faint).child(relative_time(tip_time))
    }
}

impl ParentElement for BranchRow {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.trailing.extend(elements);
    }
}

impl RenderOnce for BranchRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let on_click = self.on_click;
        let mut row = pressable((self.id, self.ix), self.selected, cx)
            .flex()
            .items_center()
            .gap(px(10.0))
            .h(px(self.height))
            .w_full()
            .px(px(8.0))
            .rounded(px(6.0))
            .text_size(px(13.0))
            .child(
                icon(self.icon).size(px(14.0)).text_color(if self.bright { theme.text } else { theme.text_faint }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_color(if self.bright { theme.text } else { theme.text_muted })
                    .child(self.name),
            )
            .children(self.trailing)
            .on_click(move |_, w, cx| on_click(w, cx));
        if let Some(f) = self.on_right_click {
            row = row.on_mouse_down(MouseButton::Right, move |e, w, cx| f(e.position, w, cx));
        }
        row
    }
}
