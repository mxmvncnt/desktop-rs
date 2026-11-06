//! The Conversation tab: the description, then comments and review threads in time order, then a box for a new comment.

use std::rc::Rc;

use gd_forge::{Comment, PullRequest, Thread};
use gd_ui::{Button, Theme, empty_state};
use gpui::{
    App, Div, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};

use super::comment_box::CommentBox;
use super::comment_card::comment_card;
use super::ops::Reply;
use super::state::{PullTab, PullsStore};

#[derive(IntoElement)]
pub struct ConversationView {
    pub store: Entity<PullsStore>,
    pub pull: PullRequest,
}

/// One entry of the timeline.
enum Item<'a> {
    Comment(&'a Comment),
    Thread(&'a Thread),
}

impl RenderOnce for ConversationView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let pulls = self.store.read(cx);
        let p = &self.pull;
        let description = Comment {
            id: 0,
            author: p.author.clone(),
            body: if p.body.trim().is_empty() {
                "No description provided.".into()
            } else {
                p.body.clone()
            },
            created_at: p.created_at,
            verdict: None,
        };
        let timeline = match (&pulls.conversation, &pulls.conversation_error) {
            (Some(c), _) => {
                let mut items: Vec<Item> = c
                    .comments
                    .iter()
                    .map(Item::Comment)
                    .chain(c.threads.iter().map(Item::Thread))
                    .collect();
                items.sort_by_key(|i| match i {
                    Item::Comment(c) => c.created_at,
                    Item::Thread(t) => t.comments.first().and_then(|c| c.created_at),
                });
                items
                    .into_iter()
                    .map(|i| match i {
                        Item::Comment(c) => {
                            card(theme).child(comment_card(c, theme)).into_any_element()
                        }
                        Item::Thread(t) => self.thread(t, cx).into_any_element(),
                    })
                    .collect()
            }
            (None, Some(problem)) => {
                let (title, body) =
                    problem.describe(crate::features::accounts::Subject::PullRequests);
                vec![
                    div()
                        .text_color(theme.danger)
                        .child(format!("{title}. {body}"))
                        .into_any_element(),
                ]
            }
            (None, None) => vec![empty_state("Loading the conversation…", cx).into_any_element()],
        };
        let post = self.store.clone();
        div()
            .id("pull-conversation")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&pulls.conversation_scroll)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .px(px(24.0))
                    .py(px(16.0))
                    .max_w(px(900.0))
                    .child(card(theme).child(comment_card(&description, theme)))
                    .children(timeline)
                    .child(
                        CommentBox {
                            id: "pull-comment",
                            input: pulls.inputs.comment.clone(),
                            placeholder: match pulls.ready() {
                                Some(t) => format!("Comment as {}", t.username),
                                None => "Add a comment".into(),
                            },
                            action: "Comment",
                            posting: pulls.posting,
                            on_post: Rc::new(move |_, cx| {
                                post.update(cx, |p, cx| p.post(Reply::Conversation, cx))
                            }),
                            on_cancel: None,
                        }
                        .build(cx),
                    ),
            )
    }
}

impl ConversationView {
    /// A review thread: where it is (a link to the Files tab), its comments and a reply box.
    fn thread(&self, t: &Thread, cx: &App) -> Div {
        let theme = Theme::of(cx);
        let pulls = self.store.read(cx);
        let open = pulls.replying.as_deref() == Some(t.id.as_str());
        let lines = match (t.start_line, t.line.or(t.original_line)) {
            (Some(s), Some(e)) => format!("lines {s}–{e}"),
            (None, Some(e)) => format!("line {e}"),
            _ => String::new(),
        };
        let files = self.store.clone();
        let (reply, thread) = (self.store.clone(), t.clone());
        card(theme)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .text_size(px(12.0))
                    .child(
                        div()
                            .id(ElementId::Name(format!("pull-thread-file-{}", t.id).into()))
                            .cursor_pointer()
                            .font_family(theme.font_mono.clone())
                            .text_color(theme.accent)
                            .hover_style(|s| s.underline())
                            .child(format!("{} {lines}", t.path))
                            .on_click(move |_, _, cx| {
                                files.update(cx, |p, cx| p.set_tab(PullTab::Files, cx))
                            }),
                    )
                    .when(t.line.is_none(), |d| {
                        d.child(div().text_color(theme.text_faint).child("Outdated"))
                    })
                    .when(t.resolved, |d| {
                        d.child(div().text_color(theme.text_faint).child("Resolved"))
                    }),
            )
            .children(t.comments.iter().map(|c| comment_card(c, theme)))
            .child(if open {
                let (post, cancel) = (self.store.clone(), self.store.clone());
                CommentBox {
                    id: "pull-reply",
                    input: pulls.inputs.reply.clone(),
                    placeholder: "Reply…".into(),
                    action: "Reply",
                    posting: pulls.posting,
                    on_post: Rc::new(move |_, cx| {
                        let thread = thread.clone();
                        post.update(cx, |p, cx| p.post(Reply::Thread(thread), cx))
                    }),
                    on_cancel: Some(Rc::new(move |_, cx| {
                        cancel.update(cx, |p, cx| p.start_reply(None, cx))
                    })),
                }
                .build(cx)
                .into_any_element()
            } else {
                div()
                    .flex()
                    .child(
                        Button::new(ElementId::Name(format!("pull-reply-{}", t.id).into()))
                            .label("Reply")
                            .ghost()
                            .on_click(move |_, _, cx| {
                                let id = thread.id.clone();
                                reply.update(cx, |p, cx| p.start_reply(Some(id), cx))
                            }),
                    )
                    .into_any_element()
            })
    }
}

fn card(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .p(px(14.0))
        .rounded(px(10.0))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface_card)
}
