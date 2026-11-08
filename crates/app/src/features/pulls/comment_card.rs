//! One comment: author, time, verdict and body (Markdown). A plain fn: it is also drawn inside diff rows.

use gd_forge::Comment;
use gd_ui::Theme;
use gpui::{Div, FontWeight, ParentElement, Styled, div, prelude::*, px};

use crate::components::markdown;
use crate::utils::text::relative_time;

pub fn comment_card(c: &Comment, theme: &Theme) -> Div {
    let author = c
        .author
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_else(|| "ghost".into());
    let when = c
        .created_at
        .map(|t| relative_time(t.timestamp()))
        .unwrap_or_default();
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .text_size(px(12.0))
                .child(div().font_weight(FontWeight::MEDIUM).child(author))
                .children(c.verdict.clone().map(|v| {
                    let color = if v == "approved" {
                        theme.success
                    } else {
                        theme.warning
                    };
                    div().text_color(color).child(v)
                }))
                .child(div().text_color(theme.text_faint).child(when)),
        )
        .when(!c.body.trim().is_empty(), |d| {
            d.child(
                markdown(&c.id.to_string(), &c.body, theme)
                    .text_size(px(13.0))
                    .line_height(px(19.0))
                    .text_color(theme.text),
            )
        })
}
