//! Right-click menu: a `deferred` full-window layer with a backdrop and a card
//! positioned at the mouse, clamped to the window. Stateless: the owner renders
//! it only while open and handles keyboard selection.

use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, ParentElement, Pixels, Point, RenderOnce, SharedString, Styled, Window, div, prelude::*, px};

use super::{Popover, icon, pressable};
use crate::assets::Icon;
use crate::theme::Theme;

type Callback = Rc<dyn Fn(&mut Window, &mut App)>;

/// One selectable row of a [`ContextMenu`].
pub struct MenuItem {
    pub label: SharedString,
    pub icon: Option<Icon>,
    pub danger: bool,
    pub disabled: bool,
    pub on_select: Option<Callback>,
}

impl MenuItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self { label: label.into(), icon: None, danger: false, disabled: false, on_select: None }
    }
    pub fn icon(mut self, i: Icon) -> Self {
        self.icon = Some(i);
        self
    }
    /// Destructive item: danger-colored label and icon.
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
    pub fn on_select(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

pub enum MenuEntry {
    Item(MenuItem),
    Separator,
}

const MENU_WIDTH: f32 = 236.0;
const MENU_ITEM_H: f32 = 28.0;
const MENU_PAD: f32 = 4.0;
const SEPARATOR_H: f32 = 9.0;

/// Floating menu at a window position (right-click). The owner highlights
/// `selected` (an index into the entries) for keyboard navigation.
#[derive(IntoElement)]
pub struct ContextMenu {
    id: ElementId,
    position: Point<Pixels>,
    entries: Vec<MenuEntry>,
    selected: Option<usize>,
    on_dismiss: Option<Callback>,
}

impl ContextMenu {
    pub fn new(id: impl Into<ElementId>, position: Point<Pixels>) -> Self {
        Self { id: id.into(), position, entries: Vec::new(), selected: None, on_dismiss: None }
    }
    pub fn item(mut self, item: MenuItem) -> Self {
        self.entries.push(MenuEntry::Item(item));
        self
    }
    pub fn separator(mut self) -> Self {
        self.entries.push(MenuEntry::Separator);
        self
    }
    pub fn entries(mut self, entries: impl IntoIterator<Item = MenuEntry>) -> Self {
        self.entries.extend(entries);
        self
    }
    pub fn selected(mut self, ix: Option<usize>) -> Self {
        self.selected = ix;
        self
    }
    /// Called when the backdrop is clicked (left or right).
    pub fn on_dismiss(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for ContextMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        // Keep the card inside the window (flip up / shift left near edges).
        // `position` is in window coordinates; under client-side decorations
        // the content sits inside a transparent shadow inset, so translate.
        let inset = window.client_inset().unwrap_or(px(0.0));
        let position = gpui::point(self.position.x - inset, self.position.y - inset);
        let mut view = window.viewport_size();
        view.width -= inset * 2.0;
        view.height -= inset * 2.0;
        let height: f32 = MENU_PAD * 2.0
            + self
                .entries
                .iter()
                .map(|e| match e {
                    MenuEntry::Item(_) => MENU_ITEM_H,
                    MenuEntry::Separator => SEPARATOR_H,
                })
                .sum::<f32>();
        let x = position.x.min(view.width - px(MENU_WIDTH + 8.0)).max(px(8.0));
        let y = if position.y + px(height + 8.0) > view.height {
            (position.y - px(height)).max(px(8.0))
        } else {
            position.y
        };

        let mut card = div().p(px(MENU_PAD)).flex().flex_col();
        for (ix, entry) in self.entries.into_iter().enumerate() {
            card = card.child(match entry {
                MenuEntry::Separator => div()
                    .h(px(SEPARATOR_H))
                    .flex()
                    .items_center()
                    .child(div().h(px(1.0)).w_full().bg(theme.border))
                    .into_any_element(),
                MenuEntry::Item(item) => {
                    let fg = if item.danger { theme.danger } else { theme.text };
                    let icon_fg = if item.danger { theme.danger } else { theme.text_muted };
                    let selected = self.selected == Some(ix) && !item.disabled;
                    let mut row = pressable(("menu-item", ix), selected, cx)
                        .flex()
                        .items_center()
                        .gap(px(9.0))
                        .h(px(MENU_ITEM_H))
                        .px(px(8.0))
                        .rounded(px(6.0))
                        .text_size(px(13.0))
                        .text_color(fg)
                        .child(
                            div().flex_none().w(px(15.0)).children(
                                item.icon.map(|i| icon(i).size(px(15.0)).text_color(icon_fg)),
                            ),
                        )
                        .child(div().flex_1().min_w_0().truncate().child(item.label));
                    if item.disabled {
                        row = row.opacity(0.4).cursor_default();
                    } else if let Some(f) = item.on_select {
                        row = row.on_click(move |_, w, cx| f(w, cx));
                    }
                    row.into_any_element()
                }
            });
        }

        let mut menu = Popover::new(self.id, gpui::point(x, y), MENU_WIDTH, card);
        if let Some(f) = self.on_dismiss {
            menu = menu.on_dismiss(move |_, w, cx| f(w, cx));
        }
        menu
    }
}
