//! Suggestions under the user filter while typing: the users seen in the loaded pipelines whose login or name contains the text.
//! The forges only filter on an exact login, so picking a suggestion (or Enter) is what turns the text into one.

use std::collections::HashSet;

use gd_forge::User;
use gd_ui::{Theme, overlay_card, pressable};
use gpui::{
    App, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce, StatefulInteractiveElement, Styled, Window, deferred,
    div, px,
};
use gpui_elements::editable_text::EditableTextState;

use super::state::CiStore;

const MAX_SUGGESTIONS: usize = 8;
const WIDTH: f32 = 260.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub login: String,
    pub name: Option<String>,
}

/// The users that contain `typed` (ignoring case) in their login or name, those that start with it first, each login once.
pub fn suggest<'a>(users: impl IntoIterator<Item = &'a User>, typed: &str) -> Vec<Suggestion> {
    let typed = typed.trim().to_lowercase();
    if typed.is_empty() {
        return Vec::new();
    }
    let mut seen = HashSet::new();
    let mut found: Vec<(bool, Suggestion)> = users
        .into_iter()
        .filter(|u| seen.insert(u.username.to_lowercase()))
        .filter_map(|u| {
            let (login, name) = (u.username.to_lowercase(), u.name.as_deref().unwrap_or("").to_lowercase());
            (login.contains(&typed) || name.contains(&typed))
                .then(|| (login.starts_with(&typed) || name.starts_with(&typed), Suggestion { login: u.username.clone(), name: u.name.clone() }))
        })
        .collect();
    found.sort_by_key(|(prefix, s)| (!prefix, s.login.to_lowercase()));
    found.into_iter().map(|(_, s)| s).take(MAX_SUGGESTIONS).collect()
}

/// What Enter applies: the suggestion whose login is the text, else the first suggestion, else the text as typed (a login not seen yet).
pub fn on_enter(suggestions: &[Suggestion], typed: &str) -> String {
    let typed = typed.trim();
    suggestions
        .iter()
        .find(|s| s.login.eq_ignore_ascii_case(typed))
        .or(suggestions.first())
        .map_or_else(|| typed.to_string(), |s| s.login.clone())
}

#[derive(IntoElement)]
pub struct UserMenu {
    pub store: Entity<CiStore>,
    /// The user field: picking a suggestion writes the login there.
    pub input: Entity<EditableTextState>,
    pub suggestions: Vec<Suggestion>,
}

impl RenderOnce for UserMenu {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let close = self.store.clone();
        let rows = self.suggestions.into_iter().enumerate().map(|(ix, s)| {
            let (store, input, login) = (self.store.clone(), self.input.clone(), s.login.clone());
            // The first one is what Enter applies.
            pressable(("ci-user-pick", ix), ix == 0, cx)
                .flex()
                .items_center()
                .gap(px(8.0))
                .h(px(28.0))
                .px(px(10.0))
                .rounded(px(gd_ui::theme::CONTROL_RADIUS))
                .child(div().flex_none().child(s.login))
                .children(s.name.map(|name| div().flex_1().truncate().text_size(px(12.0)).text_color(theme.text_faint).child(name)))
                .on_click(move |_, _, cx| {
                    input.update(cx, |s, cx| s.emplace(&login, cx));
                    store.update(cx, |s, cx| s.apply_user(login.clone(), cx));
                })
        });
        // The backdrop catches the click that dismisses the menu wherever it lands (see `BranchMenu`).
        deferred(
            div()
                .absolute()
                .top_0()
                .left_0()
                .child(div().absolute().top(px(-3000.0)).left(px(-3000.0)).size(px(6000.0)).occlude().on_mouse_down(
                    MouseButton::Left,
                    move |_, _, cx| {
                        close.update(cx, |s, cx| {
                            s.user_menu = false;
                            cx.notify()
                        })
                    },
                ))
                .child(div().absolute().top(px(36.0)).left_0().w(px(WIDTH)).child(overlay_card(cx).occlude().p(px(4.0)).children(rows))),
        )
        .priority(2)
    }
}

#[cfg(test)]
#[path = "tests/user_menu.test.rs"]
mod tests;
