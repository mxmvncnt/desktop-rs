//! `Workspace::review_notes`: the review threads of the shown file under their lines (outdated ones at the top), the bar
//! that offers a comment on the selected text, and the box for a comment on the picked lines, as `RowNotes` for the diff
//! (see `features/diff/notes.rs`).

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

use gd_forge::{Side, Thread};
use gd_git::{Diff, DiffLineKind};
use gd_ui::{Button, Icon, Theme};
use gpui::{
    AnyElement, App, ElementId, Entity, IntoElement, ParentElement, Styled, div, prelude::*, px,
};

use super::comment_box::CommentBox;
use super::comment_card::comment_card;
use super::ops::Reply;
use super::state::{Pick, PullTab, PullsStore};
use super::thread_header::thread_header;
use crate::features::diff::RowNotes;
use crate::workspace::Workspace;

/// The bar under the last line of the selected text.
struct SelectionBar {
    last: usize,
    label: String,
    pick: Pick,
}

impl Workspace {
    /// Notes of the review diff on the Files tab; `None` elsewhere (a commit of the Commits tab is not commented on).
    pub(crate) fn review_notes(&self, diff: &Diff, cx: &gpui::Context<Self>) -> Option<RowNotes> {
        let store = self.pulls.clone();
        let pulls = store.read(cx);
        if !pulls.open || pulls.tab != PullTab::Files || pulls.detail.is_none() {
            return None;
        }
        let path = self.repo.as_ref()?.read(cx).review.path.clone()?;
        let mut threads: HashMap<usize, Vec<Thread>> = HashMap::new();
        for t in pulls.conversation.iter().flat_map(|c| &c.threads) {
            if t.path != path {
                continue;
            }
            // An outdated thread has no line in this diff: it goes under the first row.
            if let Some(ix) = anchor(diff, t).or((t.line.is_none()).then_some(0)) {
                threads.entry(ix).or_default().push(t.clone());
            }
        }
        let pick = pulls.pick.clone().filter(|p| p.path == path);
        // The text selected in the diff, once the button is up, offers a comment on its lines.
        let bar = self
            .diff_view
            .text_sel
            .filter(|s| !self.diff_view.text_drag && !s.is_empty())
            .and_then(|s| {
                let (from, to) = s.ordered();
                let number = |ix: usize| diff.lines.get(ix).and_then(|l| l.new_line.or(l.old_line));
                let start = (from.line..=to.line).find(|&ix| number(ix).is_some())?;
                let end = (from.line..=to.line)
                    .rev()
                    .find(|&ix| number(ix).is_some())?;
                let (a, b) = (number(start)?, number(end)?);
                Some(SelectionBar {
                    last: to.line,
                    label: if start == end {
                        format!("Comment on line {a}")
                    } else {
                        format!("Comment on lines {a}–{b}")
                    },
                    pick: Pick {
                        path: path.clone(),
                        start,
                        end,
                    },
                })
            });
        let mut key = DefaultHasher::new();
        let mut anchored: Vec<_> = threads
            .iter()
            .flat_map(|(ix, ts)| {
                ts.iter()
                    .map(|t| (*ix, t.id.as_str(), t.comments.len(), pulls.collapsed(t)))
            })
            .collect();
        anchored.sort();
        let bar_key = bar.as_ref().map(|b| (b.last, &b.label));
        (
            &path,
            anchored,
            &pick,
            &pulls.replying,
            pulls.posting,
            bar_key,
        )
            .hash(&mut key);
        let ws = cx.entity();
        let pick_store = store.clone();
        let pick_path = path.clone();
        Some(RowNotes {
            picked: pick.as_ref().map(|p| p.start..=p.end),
            on_pick: Rc::new(move |ix, extend, _, cx| {
                let path = pick_path.clone();
                pick_store.update(cx, |p, cx| p.pick_line(path, ix, extend, cx))
            }),
            below: Rc::new(move |ix, cx| {
                let composer = pick.as_ref().is_some_and(|p| p.end == ix);
                let here = threads.get(&ix);
                let bar = bar.as_ref().filter(|b| b.last == ix);
                if here.is_none() && bar.is_none() && !composer {
                    return None;
                }
                Some(block(
                    here.map(Vec::as_slice).unwrap_or(&[]),
                    bar,
                    composer,
                    &store,
                    &ws,
                    cx,
                ))
            }),
            key: key.finish(),
        })
    }
}

/// The line of the diff a thread is on: an added or context line by its new number, a deleted one by its old number.
fn anchor(diff: &Diff, t: &Thread) -> Option<usize> {
    let line = t.line?;
    diff.lines.iter().position(|l| match t.side {
        Side::Right => l.kind != DiffLineKind::Delete && l.new_line == Some(line),
        Side::Left => l.kind == DiffLineKind::Delete && l.old_line == Some(line),
    })
}

/// The threads on a line, the bar of the selected text, then the box for a new comment when the pick ends there.
fn block(
    threads: &[Thread],
    bar: Option<&SelectionBar>,
    composer: bool,
    store: &Entity<PullsStore>,
    ws: &Entity<Workspace>,
    cx: &App,
) -> AnyElement {
    let theme = Theme::of(cx);
    let pulls = store.read(cx);
    let card = || {
        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .max_w(px(760.0))
            .p(px(12.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.surface_card)
            .font_family(theme.font_sans.clone())
    };
    let thread_cards = threads.iter().map(|t| {
        let collapsed = pulls.collapsed(t);
        let open = pulls.replying.as_deref() == Some(t.id.as_str());
        let (reply_store, thread) = (store.clone(), t.clone());
        card()
            .child(thread_header(t, collapsed, store, theme))
            .when(!collapsed, |d| {
                d.children(t.comments.iter().map(|c| comment_card(c, theme)))
                    .child(if open {
                        let (post, cancel) = (store.clone(), store.clone());
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
                                        reply_store.update(cx, |p, cx| p.start_reply(Some(id), cx))
                                    }),
                            )
                            .into_any_element()
                    })
            })
    });
    let bar = bar.map(|b| {
        let (ws, pick) = (ws.clone(), b.pick.clone());
        div().flex().child(
            Button::new("pull-comment-selection")
                .label(b.label.clone())
                .on_click(move |_, _, cx| {
                    let pick = pick.clone();
                    ws.update(cx, |w, cx| {
                        w.diff_view.text_sel = None;
                        w.pulls.update(cx, |p, cx| {
                            p.pick = Some(pick);
                            p.replying = None;
                            cx.notify()
                        });
                        cx.notify();
                    })
                }),
        )
    });
    let composer = composer.then(|| {
        let (post, cancel) = (ws.clone(), store.clone());
        let lines = pulls.pick.as_ref().map_or(1, |p| p.end - p.start + 1);
        card()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .text_size(px(12.0))
                    .text_color(theme.text_muted)
                    .child(
                        gd_ui::icon(Icon::Pen)
                            .size(px(12.0))
                            .text_color(theme.text_muted),
                    )
                    .child(if lines == 1 {
                        "Comment on this line".to_string()
                    } else {
                        format!("Comment on {lines} lines")
                    }),
            )
            .child(
                CommentBox {
                    id: "pull-line-comment",
                    input: pulls.inputs.line.clone(),
                    placeholder: "Leave a comment".into(),
                    action: "Comment",
                    posting: pulls.posting,
                    on_post: Rc::new(move |_, cx| {
                        post.update(cx, |ws, cx| ws.post_line_comment(cx))
                    }),
                    on_cancel: Some(Rc::new(move |_, cx| {
                        cancel.update(cx, |p, cx| p.cancel_pick(cx))
                    })),
                }
                .build(cx),
            )
    });
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .w_full()
        .px(px(16.0))
        .py(px(10.0))
        .border_y_1()
        .border_color(theme.border)
        .bg(theme.bg)
        .whitespace_normal()
        .font_family(theme.font_sans.clone())
        .text_size(px(13.0))
        .line_height(px(19.0))
        .children(thread_cards)
        .children(bar)
        .children(composer)
        .into_any_element()
}

#[cfg(test)]
#[path = "tests/review_notes.test.rs"]
mod tests;
