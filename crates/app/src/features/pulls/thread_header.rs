//! The header row of a review thread, on the Conversation tab and in the diff: a chevron that minimizes the thread, the
//! first comment's author and, when minimized, the first line of its body. A plain fn: it is also drawn inside diff rows.

use gd_forge::Thread;
use gd_ui::{Icon, Theme};
use gpui::{
    ElementId, Entity, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled, div, prelude::*, px,
};

use super::state::PullsStore;

pub fn thread_header(
    t: &Thread,
    collapsed: bool,
    store: &Entity<PullsStore>,
    theme: &Theme,
) -> impl IntoElement + use<> {
    let first = t.comments.first();
    let author = first
        .and_then(|c| c.author.as_ref())
        .map_or("ghost", |u| u.username.as_str())
        .to_string();
    let preview = first
        .and_then(|c| c.body.lines().find(|l| !l.trim().is_empty()))
        .unwrap_or_default()
        .trim()
        .to_string();
    let (store, id) = (store.clone(), t.id.clone());
    div()
        .id(ElementId::Name(format!("pull-thread-toggle-{id}").into()))
        .flex()
        .items_center()
        .gap(px(8.0))
        .cursor_pointer()
        .text_size(px(12.0))
        .text_color(theme.text_muted)
        .child(
            gd_ui::icon(if collapsed {
                Icon::ChevronRight
            } else {
                Icon::ChevronDown
            })
            .size(px(12.0))
            .text_color(theme.text_muted),
        )
        .child(div().text_color(theme.text).child(author))
        .child(div().flex_1().min_w_0().truncate().child(if collapsed {
            preview
        } else {
            String::new()
        }))
        .when(t.comments.len() > 1, |d| {
            d.child(format!("{} comments", t.comments.len()))
        })
        .when(t.line.is_none(), |d| {
            d.child(match t.original_line {
                Some(n) => format!("Outdated · line {n}"),
                None => "Outdated".into(),
            })
        })
        .when(t.resolved, |d| d.child("Resolved"))
        .on_click(move |_, _, cx| store.update(cx, |p, cx| p.toggle_thread(&id, cx)))
}
