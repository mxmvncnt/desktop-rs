//! Top of the pipelines list: title, the account in use, the status tabs, the branch and user filters and Refresh.

use gd_forge::Status;
use gd_ui::{Button, Icon, Tabs, Theme, icon_button};
use gpui_elements::editable_text::actions::Enter;
use gpui::{App, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled, Window, div, prelude::*, px};

use super::branch_menu::{BranchMenu, branch_names};
use super::user_menu::{UserMenu, on_enter, suggest};
use super::state::CiStore;
use crate::components::text_field;
use crate::workspace::Workspace;

/// The status tabs, in order.
const STATUS_TABS: [(&str, Option<Status>); 4] =
    [("All", None), ("Running", Some(Status::Running)), ("Failed", Some(Status::Failed)), ("Success", Some(Status::Success))];

#[derive(IntoElement)]
pub struct PipelinesHeader {
    pub store: Entity<CiStore>,
    pub ws: Entity<Workspace>,
}

impl RenderOnce for PipelinesHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let ci = self.store.read(cx);
        let target = ci.ready();
        let selected = STATUS_TABS.iter().position(|(_, s)| *s == ci.status).unwrap_or(0);
        let refreshing = ci.refreshing;
        let account = target.map(|t| format!("via {} · {}", t.account_name, t.host));
        let current_branch = target.and_then(|t| t.branch.clone());
        let can_filter_user = target.is_some_and(|t| t.forge.capabilities().filter_user);
        let inputs = self.ws.read(cx).ci_inputs.clone();
        let (ws, store, menu_store, refresh_ws) = (self.ws.clone(), self.store.clone(), self.store.clone(), self.ws.clone());
        let user_menu = ci.user_menu.then(|| suggest(ci.known_users(), inputs.user.read(cx).as_str())).filter(|s| !s.is_empty()).map(|suggestions| UserMenu {
            store: self.store.clone(),
            input: inputs.user.clone(),
            suggestions,
        });
        let enter_store = self.store.clone();
        let enter_input = inputs.user.clone();
        let branch_menu = ci.branch_menu.then(|| BranchMenu {
            store: self.store.clone(),
            input: inputs.branch.clone(),
            typed: inputs.branch.read(cx).as_str().trim().to_string(),
            branches: self.ws.read(cx).repo.as_ref().map(|r| branch_names(&r.read(cx).branches, current_branch.as_deref())).unwrap_or_default(),
            current: current_branch.clone(),
        });
        let labeled = |label: &'static str| div().flex().items_center().gap(px(8.0)).child(div().text_size(px(12.0)).text_color(theme.text_muted).child(label));

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
                            .child(div().text_size(px(18.0)).font_weight(gpui::FontWeight::MEDIUM).child("Pipelines"))
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
                                            .id("ci-change-account")
                                            .cursor_pointer()
                                            .text_color(theme.accent)
                                            .hover_style(|s| s.underline())
                                            .child("Change")
                                            .on_click(move |_, _, cx| ws.update(cx, |ws, cx| ws.open_repo_account_settings(cx))),
                                    )
                            })),
                    )
                    .child(
                        Button::new("ci-refresh").icon(Icon::Refresh).label("Refresh").disabled(refreshing).on_click(move |_, _, cx| {
                            refresh_ws.update(cx, |ws, cx| ws.sync_ci(cx))
                        }),
                    ),
            )
            .when(target.is_some(), |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .child(
                            div().w(px(340.0)).child(
                                STATUS_TABS
                                    .iter()
                                    .fold(Tabs::new("ci-status-tabs", selected), |tabs, (label, _)| tabs.tab(*label, None))
                                    .on_select(move |ix, _, cx| store.update(cx, |s, cx| s.set_status(STATUS_TABS[ix].1, cx))),
                            ),
                        )
                        .child(
                            labeled("Branch").child(
                                div()
                                    .relative()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(div().w(px(180.0)).child(text_field("ci-branch-filter", &inputs.branch, "All branches", cx)))
                                    .child(icon_button("ci-branch-menu-toggle", Icon::ChevronDown).on_click(move |_, _, cx| {
                                        menu_store.update(cx, |s, cx| {
                                            s.branch_menu = !s.branch_menu;
                                            cx.notify()
                                        })
                                    }))
                                    .children(branch_menu),
                            ),
                        )
                        .when(can_filter_user, |d| {
                            d.child(
                                labeled("User").child(
                                    div()
                                        .relative()
                                        .w(px(150.0))
                                        .capture_action(move |_: &Enter, _, cx| {
                                            // Enter applies what the suggestions say (see `on_enter`).
                                            cx.stop_propagation();
                                            let typed = enter_input.read(cx).as_str().trim().to_string();
                                            let login = on_enter(&suggest(enter_store.read(cx).known_users(), &typed), &typed);
                                            enter_input.update(cx, |s, cx| s.emplace(&login, cx));
                                            enter_store.update(cx, |s, cx| s.apply_user(login, cx));
                                        })
                                        .child(text_field("ci-user-filter", &inputs.user, "Anyone", cx))
                                        .children(user_menu),
                                ),
                            )
                        }),
                )
            })
    }
}
