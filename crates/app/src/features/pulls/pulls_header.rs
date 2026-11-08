//! Top of the pull requests list: title, the account in use, the search field, the state tabs and Refresh.

use gd_forge::{Forge, PullState};
use gd_ui::{Button, Icon, Tabs, Theme};
use gpui::{
    App, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, point, prelude::*, px,
};

use gpui_elements::editable_text::actions::Enter;

use super::state::PullsStore;
use crate::components::text_field;
use crate::workspace::Workspace;

/// The state tabs, in order.
const STATE_TABS: [(&str, Option<PullState>); 4] = [
    ("Open", Some(PullState::Open)),
    ("Merged", Some(PullState::Merged)),
    ("Closed", Some(PullState::Closed)),
    ("All", None),
];

#[derive(IntoElement)]
pub struct PullsHeader {
    pub store: Entity<PullsStore>,
    pub ws: Entity<Workspace>,
}

impl RenderOnce for PullsHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let pulls = self.store.read(cx);
        let target = pulls.ready();
        let selected = STATE_TABS
            .iter()
            .position(|(_, s)| *s == pulls.state())
            .unwrap_or(0);
        let account = target.map(|t| format!("via {} · {}", t.account_name, t.host));
        let (ws, store, refresh_ws, filters_ws) = (
            self.ws.clone(),
            self.store.clone(),
            self.ws.clone(),
            self.ws.clone(),
        );
        let (input, enter) = (pulls.inputs.query.clone(), self.store.clone());
        // GitHub runs the query itself; elsewhere what `matches` can't tell is said.
        let note = match (&pulls.query_error, target.map(|t| t.forge)) {
            (Some(e), _) => Some((format!("Can't read the query: {e}"), theme.danger)),
            (None, Some(f)) if f != Forge::GitHub => {
                let unknown = pulls.query.unsupported();
                (!unknown.is_empty()).then(|| {
                    (
                        format!(
                            "Not understood here, so not filtered on: {}",
                            unknown.join(" ")
                        ),
                        theme.warning,
                    )
                })
            }
            _ => None,
        };
        div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(14.0))
            .px(px(24.0))
            .pt(px(14.0))
            .pb(px(12.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_size(px(18.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .child("Pull requests"),
                            )
                            .children(account.map(|account| {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .text_size(px(12.0))
                                    .text_color(theme.text_muted)
                                    .child(account)
                                    .child(
                                        div()
                                            .id("pulls-change-account")
                                            .cursor_pointer()
                                            .text_color(theme.accent)
                                            .hover_style(|s| s.underline())
                                            .child("Change")
                                            .on_click(move |_, _, cx| {
                                                ws.update(cx, |ws, cx| {
                                                    ws.open_repo_account_settings(cx)
                                                })
                                            }),
                                    )
                            })),
                    )
                    .child(
                        Button::new("pulls-refresh")
                            .icon(Icon::Refresh)
                            .label("Refresh")
                            .disabled(pulls.refreshing)
                            .on_click(move |_, _, cx| {
                                refresh_ws.update(cx, |ws, cx| ws.sync_pulls(cx))
                            }),
                    ),
            )
            .when(target.is_some(), |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    Button::new("pulls-filters")
                                        .icon(Icon::ChevronDown)
                                        .label("Filters")
                                        .on_click(move |e, w, cx| {
                                            // Just under the button.
                                            let position = e.position() + point(px(0.0), px(16.0));
                                            filters_ws.update(cx, |ws, cx| {
                                                ws.open_pulls_filters(position, w, cx)
                                            })
                                        }),
                                )
                                .child(
                                    div()
                                        .w(px(720.0))
                                        .capture_action(move |_: &Enter, _, cx| {
                                            cx.stop_propagation();
                                            let text = input.read(cx).as_str().to_string();
                                            enter.update(cx, |s, cx| s.apply_query(text, cx))
                                        })
                                        .child(text_field(
                                            "pulls-query",
                                            &pulls.inputs.query,
                                            "is:pr state:open author:@me",
                                            cx,
                                        )),
                                ),
                        )
                        .children(note.map(|(text, color)| {
                            div().text_size(px(12.0)).text_color(color).child(text)
                        })),
                )
                .child(
                    div().w(px(340.0)).child(
                        STATE_TABS
                            .iter()
                            .fold(
                                Tabs::new("pulls-state-tabs", selected),
                                |tabs, (label, _)| tabs.tab(*label, None),
                            )
                            .on_select(move |ix, _, cx| {
                                store.update(cx, |s, cx| s.set_state(STATE_TABS[ix].1, cx))
                            }),
                    ),
                )
            })
    }
}
