//! The empty state of the page: nothing to show, why, and the button that fixes it.

use gd_ui::{Button, Icon, Theme, icon};
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, prelude::*, px};

use super::target::Problem;
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct ProblemView {
    pub problem: Problem,
    pub ws: Entity<Workspace>,
}

impl RenderOnce for ProblemView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let (title, body) = self.problem.describe();
        let accounts = matches!(self.problem, Problem::NoAccount(_) | Problem::Unauthorized | Problem::NotFound);
        let pick = matches!(self.problem, Problem::Unsupported(_) | Problem::NotFound);
        let retry = !matches!(self.problem, Problem::NoRemote);
        let refreshing = self.ws.read(cx).ci.read(cx).refreshing;
        let (open_ws, pick_ws, retry_ws) = (self.ws.clone(), self.ws.clone(), self.ws);
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(10.0))
            .px(px(24.0))
            .child(icon(Icon::Pipeline).size(px(28.0)).text_color(theme.text_faint))
            .child(div().text_size(px(15.0)).font_weight(gpui::FontWeight::MEDIUM).child(title))
            .child(div().max_w(px(440.0)).text_center().text_color(theme.text_muted).child(body))
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .pt(px(6.0))
                    .when(accounts, |d| {
                        d.child(Button::new("ci-open-accounts").primary().label("Open Accounts").on_click(move |_, _, cx| {
                            open_ws.update(cx, |ws, cx| ws.open_accounts_settings(cx))
                        }))
                    })
                    .when(pick, |d| {
                        d.child(Button::new("ci-pick-account").label("Choose account").on_click(move |_, _, cx| {
                            pick_ws.update(cx, |ws, cx| ws.open_repo_account_settings(cx))
                        }))
                    })
                    .when(retry, |d| {
                        d.child(Button::new("ci-retry").icon(Icon::Refresh).label("Retry").disabled(refreshing).on_click(move |_, _, cx| {
                            retry_ws.update(cx, |ws, cx| ws.sync_ci(cx))
                        }))
                    }),
            )
    }
}
