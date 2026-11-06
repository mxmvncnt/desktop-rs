//! The empty state of a forge page (CI, pull requests): nothing to show, why, and the button that fixes it.

use gd_ui::{Button, Callback, Icon, Theme, icon};
use gpui::{
    App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, prelude::*, px,
};

use crate::features::accounts::{Problem, Subject};
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct ProblemView {
    problem: Problem,
    subject: Subject,
    icon: Icon,
    /// The retry is running: its button is disabled.
    refreshing: bool,
    on_retry: Callback,
    ws: Entity<Workspace>,
}

impl ProblemView {
    pub fn new(
        problem: Problem,
        subject: Subject,
        refreshing: bool,
        ws: Entity<Workspace>,
        on_retry: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            problem,
            subject,
            icon: match subject {
                Subject::Pipelines => Icon::Pipeline,
                Subject::PullRequests => Icon::PullRequest,
            },
            refreshing,
            on_retry: std::rc::Rc::new(on_retry),
            ws,
        }
    }
}

impl RenderOnce for ProblemView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let (title, body) = self.problem.describe(self.subject);
        let accounts = matches!(
            self.problem,
            Problem::NoAccount(_) | Problem::Unauthorized | Problem::NotFound
        );
        let pick = matches!(self.problem, Problem::Unsupported(_) | Problem::NotFound);
        let retry = !matches!(self.problem, Problem::NoRemote);
        let (open_ws, pick_ws, on_retry) = (self.ws.clone(), self.ws, self.on_retry);
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(10.0))
            .px(px(24.0))
            .child(icon(self.icon).size(px(28.0)).text_color(theme.text_faint))
            .child(
                div()
                    .text_size(px(15.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(title),
            )
            .child(
                div()
                    .max_w(px(440.0))
                    .text_center()
                    .text_color(theme.text_muted)
                    .child(body),
            )
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .pt(px(6.0))
                    .when(accounts, |d| {
                        d.child(
                            Button::new("problem-open-accounts")
                                .primary()
                                .label("Open Accounts")
                                .on_click(move |_, _, cx| {
                                    open_ws.update(cx, |ws, cx| ws.open_accounts_settings(cx))
                                }),
                        )
                    })
                    .when(pick, |d| {
                        d.child(
                            Button::new("problem-pick-account")
                                .label("Choose account")
                                .on_click(move |_, _, cx| {
                                    pick_ws.update(cx, |ws, cx| ws.open_repo_account_settings(cx))
                                }),
                        )
                    })
                    .when(retry, |d| {
                        d.child(
                            Button::new("problem-retry")
                                .icon(Icon::Refresh)
                                .label("Retry")
                                .disabled(self.refreshing)
                                .on_click(move |_, w, cx| on_retry(w, cx)),
                        )
                    }),
            )
    }
}
