//! `Workspace::render_pulls_page`: the pull requests page, which replaces everything under the toolbar while it is open.
//! A method, not a component: the Files and Commits tabs draw diffs, which need `&mut Workspace`.

use std::rc::Rc;

use gd_ui::{Button, Tabs, Theme, divider, empty_state, vdivider};
use gpui::{AnyElement, Context, Entity, IntoElement, ParentElement, Styled, div, prelude::*, px};

use super::commits_list::CommitsList;
use super::conversation_view::ConversationView;
use super::pull_row::{ROW_HEIGHT, pull_row};
use super::pull_summary::PullSummary;
use super::pulls_header::PullsHeader;
use super::state::{PullTab, PullsStore, PullsView};
use crate::components::{Breadcrumbs, Crumb, ProblemView, list};
use crate::features::accounts::{Problem, Subject};
use crate::features::diff::DiffPane;
use crate::workspace::Workspace;

impl Workspace {
    pub(crate) fn render_pulls_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let store = self.pulls.clone();
        let pulls = store.read(cx);
        let page = div().id("pulls-page").flex_1().min_h_0().flex().flex_col();
        let content = match (&pulls.target, pulls.view) {
            (None, _) => empty_state("Looking for the account…", cx)
                .flex_1()
                .into_any_element(),
            (Some(Err(problem)), _) => self.pulls_problem(problem.clone(), cx),
            (Some(Ok(_)), PullsView::List) => self.pulls_list(cx),
            (Some(Ok(_)), PullsView::Pull(number)) => {
                return page.child(self.pull_screen(number, cx)).into_any_element();
            }
        };
        page.child(PullsHeader {
            store,
            ws: cx.entity(),
        })
        .child(content)
        .into_any_element()
    }

    fn pulls_problem(&self, problem: Problem, cx: &mut Context<Self>) -> AnyElement {
        let ws = cx.entity();
        let refreshing = self.pulls.read(cx).refreshing;
        ProblemView::new(
            problem,
            Subject::PullRequests,
            refreshing,
            ws.clone(),
            move |_, cx| ws.update(cx, |ws, cx| ws.sync_pulls(cx)),
        )
        .into_any_element()
    }

    fn pulls_list(&self, cx: &mut Context<Self>) -> AnyElement {
        let store = self.pulls.clone();
        let pulls = store.read(cx);
        let theme = Theme::of(cx);
        let count = pulls.pulls.len();
        if count == 0 {
            return match (&pulls.list_error, pulls.loading) {
                (Some(problem), _) => self.pulls_problem(problem.clone(), cx),
                (None, true) => empty_state("Loading pull requests…", cx)
                    .flex_1()
                    .into_any_element(),
                (None, false) => empty_state("No pull requests match", cx)
                    .flex_1()
                    .into_any_element(),
            };
        }
        let list_store = store.clone();
        // One more row after the pull requests holds the button that loads the next page.
        let rows = list(
            "pulls-list",
            count + usize::from(pulls.more),
            move |range, _, cx| {
                let pulls = list_store.read(cx);
                range
                    .filter_map(|ix| match pulls.pulls.get(ix) {
                        Some(p) => Some(pull_row(ix, p, list_store.clone(), cx).into_any_element()),
                        None => pulls.more.then(|| {
                            let store = list_store.clone();
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .w_full()
                                .h(px(ROW_HEIGHT))
                                .child(
                                    Button::new("pulls-load-more")
                                        .label(if pulls.loading {
                                            "Loading…"
                                        } else {
                                            "Load more"
                                        })
                                        .disabled(pulls.loading)
                                        .on_click(move |_, _, cx| {
                                            store.update(cx, |s, cx| s.load_list(true, cx))
                                        }),
                                )
                                .into_any_element()
                        }),
                    })
                    .collect()
            },
        )
        .track_scroll(&pulls.list_scroll)
        .flex_1();
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .px(px(18.0))
            .children(pulls.list_error.as_ref().map(|problem| {
                let (title, body) = problem.describe(Subject::PullRequests);
                div()
                    .px(px(10.0))
                    .py(px(6.0))
                    .text_color(theme.danger)
                    .child(format!("{title}. {body}"))
            }))
            .child(rows)
            .into_any_element()
    }

    /// Breadcrumbs, summary, tabs and the tab's body.
    fn pull_screen(&mut self, number: u64, cx: &mut Context<Self>) -> AnyElement {
        let store = self.pulls.clone();
        let pulls = store.read(cx);
        let crumbs = breadcrumbs(&store, number);
        let Some(pull) = pulls.shown().cloned() else {
            let body = match &pulls.detail_error {
                Some(problem) => self.pulls_problem(problem.clone(), cx),
                None => empty_state("Loading the pull request…", cx)
                    .flex_1()
                    .into_any_element(),
            };
            return div()
                .flex_1()
                .flex()
                .flex_col()
                .child(crumbs)
                .child(body)
                .into_any_element();
        };
        let tab = pulls.tab;
        let head = pulls.pull_head().cloned();
        let comments = pulls
            .conversation
            .as_ref()
            .map(|c| c.comments.len() + c.threads.len());
        let tabs = Tabs::new("pull-tabs", tab as usize)
            .tab("Conversation", comments.map(|n| n.to_string().into()))
            .tab(
                "Commits",
                head.as_ref().map(|h| h.commits.len().to_string().into()),
            )
            .tab(
                "Files changed",
                head.as_ref().map(|h| h.files.len().to_string().into()),
            )
            .on_select({
                let store = store.clone();
                move |ix, _, cx| {
                    let tab = [PullTab::Conversation, PullTab::Commits, PullTab::Files][ix];
                    store.update(cx, |s, cx| s.set_tab(tab, cx))
                }
            });
        let body =
            match (tab, head) {
                (PullTab::Conversation, _) => ConversationView {
                    store: store.clone(),
                    pull: pull.clone(),
                }
                .into_any_element(),
                (_, None) => empty_state(
                    if pulls.head.is_none() {
                        "Fetching the pull request…"
                    } else {
                        "The pull request's commits could not be fetched"
                    },
                    cx,
                )
                .flex_1()
                .into_any_element(),
                (PullTab::Commits, Some(head)) => {
                    let diff = self.review_pane(cx);
                    div()
                        .flex()
                        .flex_1()
                        .min_h_0()
                        .child(div().flex().flex_col().flex_none().w(px(300.0)).child(
                            CommitsList {
                                store: store.clone(),
                                commits: head.commits,
                            },
                        ))
                        .child(vdivider(cx))
                        .child(diff)
                        .into_any_element()
                }
                (PullTab::Files, Some(_)) => self.review_pane(cx),
            };
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(crumbs)
            .child(PullSummary { store, pull })
            .child(
                div()
                    .flex_none()
                    .w(px(420.0))
                    .mx(px(24.0))
                    .mb(px(12.0))
                    .child(tabs),
            )
            .child(divider(cx))
            .child(body)
            .into_any_element()
    }

    /// File list and diff of the review pane.
    fn review_pane(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(repo) = self.repo.clone() else {
            return div().into_any_element();
        };
        let content = self.render_file_diff_pane(div(), &repo, DiffPane::Review, cx);
        div()
            .flex()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .child(content)
            .into_any_element()
    }
}

fn breadcrumbs(store: &Entity<PullsStore>, number: u64) -> Breadcrumbs {
    let (back, to_list) = (store.clone(), store.clone());
    Breadcrumbs {
        id: "pull-crumb",
        on_back: Rc::new(move |_, cx| back.update(cx, |s, cx| s.back(cx))),
        crumbs: vec![
            Crumb::link("Pull requests", move |_, cx| {
                to_list.update(cx, |s, cx| s.back(cx))
            }),
            Crumb::here(format!("#{number}")),
        ],
    }
}
