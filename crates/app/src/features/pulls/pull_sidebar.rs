//! Right column of the Conversation tab: merge status, checks, reviewers, assignees, labels, milestone, changes, participants.

use gd_forge::{Comment, MergeStatus, PullRequest, PullState, Verdict};
use gd_ui::{Icon, Theme, badge, icon};
use gpui::{
    AnyElement, App, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};

use super::state::PullsStore;
use crate::features::ci::status_badge;

pub const WIDTH: f32 = 290.0;

#[derive(IntoElement)]
pub struct PullSidebar {
    pub store: Entity<PullsStore>,
    pub pull: PullRequest,
}

impl RenderOnce for PullSidebar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let pulls = self.store.read(cx);
        let p = &self.pull;
        let conversation = pulls.conversation.as_ref();

        let (glyph, color, text) = match (p.state, p.merge) {
            (PullState::Merged, _) => (Icon::Fork, theme.accent, "Merged"),
            (PullState::Closed, _) => (Icon::Close, theme.danger, "Closed"),
            (_, Some(MergeStatus::Mergeable)) => (Icon::Check, theme.success, "Ready to merge"),
            (_, Some(MergeStatus::Conflicts)) => (
                Icon::Warning,
                theme.danger,
                "Has conflicts with the base branch",
            ),
            (_, Some(MergeStatus::Blocked)) => (
                Icon::Warning,
                theme.warning,
                "Merging is blocked (reviews or branch rules)",
            ),
            (_, Some(MergeStatus::ChecksPending)) => (
                Icon::Warning,
                theme.warning,
                "Checks are failing or still running",
            ),
            (_, Some(MergeStatus::Behind)) => {
                (Icon::Warning, theme.warning, "The base branch has moved on")
            }
            (_, Some(MergeStatus::Draft)) => (Icon::PullRequest, theme.text_muted, "Draft"),
            (_, Some(MergeStatus::Checking) | None) => {
                (Icon::Clock, theme.text_muted, "Checking mergeability…")
            }
        };
        let merge = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .text_color(color)
            .child(icon(glyph).size(px(14.0)).flex_none().text_color(color))
            .child(div().flex_1().min_w_0().child(text))
            .into_any_element();

        let checks: Vec<AnyElement> = pulls
            .checks
            .iter()
            .map(|pipeline| {
                let url = pipeline.web_url.clone();
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(status_badge(pipeline.status, false, theme))
                    .child(
                        div()
                            .id("pull-checks-link")
                            .cursor_pointer()
                            .text_color(theme.accent)
                            .hover_style(|s| s.underline())
                            .child(format!("#{}", pipeline.number))
                            .on_click(move |_, _, cx| cx.open_url(&url)),
                    )
                    .into_any_element()
            })
            .collect();

        let checks = if checks.is_empty() {
            vec![
                div()
                    .text_color(theme.text_faint)
                    .child("No checks")
                    .into_any_element(),
            ]
        } else {
            checks
        };

        // Reviews of reviewers who were not asked again, then the pending requests.
        let reviews = conversation
            .into_iter()
            .flat_map(|c| &c.reviews)
            .filter(|r| !p.reviewers.contains(&r.user.username))
            .map(|r| {
                let (glyph, color, text) = match r.verdict {
                    Verdict::Approved => (Icon::Check, theme.success, "approved"),
                    Verdict::ChangesRequested => (Icon::Close, theme.danger, "requested changes"),
                    Verdict::Commented => (Icon::Pen, theme.text_muted, "commented"),
                    Verdict::Dismissed => (Icon::CloseCircle, theme.text_muted, "dismissed"),
                };
                (r.user.username.as_str(), glyph, color, text)
            })
            .chain(p.reviewers.iter().map(|login| {
                (
                    login.as_str(),
                    Icon::Clock,
                    theme.text_muted,
                    "awaiting review",
                )
            }))
            .map(|(login, glyph, color, text)| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(icon(glyph).size(px(14.0)).text_color(color))
                    .child(login.to_string())
                    .child(div().text_color(theme.text_faint).child(text))
                    .into_any_element()
            })
            .collect();

        let changes = pulls.pull_head().map(|h| {
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .text_color(theme.success)
                        .child(format!("+{}", h.stats.added)),
                )
                .child(
                    div()
                        .text_color(theme.danger)
                        .child(format!("−{}", h.stats.deleted)),
                )
                .child(div().text_color(theme.text_muted).child(format!(
                    "in {} files · {} commits",
                    h.files.len(),
                    h.commits.len()
                )))
                .into_any_element()
        });

        // Whoever wrote the description or a comment, once, in the order they first spoke.
        let mut participants: Vec<&str> = Vec::new();
        let comments = conversation.into_iter().flat_map(|c| {
            c.comments
                .iter()
                .chain(c.threads.iter().flat_map(|t| &t.comments))
        });
        for author in std::iter::once(p.author.as_ref())
            .chain(comments.map(|c: &Comment| c.author.as_ref()))
            .flatten()
        {
            if !participants.contains(&author.username.as_str()) {
                participants.push(&author.username);
            }
        }

        // The names as one wrapping row of pills; nothing for no names.
        let pills = |names: Vec<&str>| -> Vec<AnyElement> {
            if names.is_empty() {
                return Vec::new();
            }
            vec![
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(6.0))
                    .children(names.into_iter().map(|n| badge(n.to_string(), cx)))
                    .into_any_element(),
            ]
        };

        div()
            .id("pull-sidebar")
            .flex()
            .flex_col()
            .flex_none()
            .w(px(WIDTH))
            .h_full()
            .overflow_y_scroll()
            .gap(px(18.0))
            .p(px(16.0))
            .border_l_1()
            .border_color(theme.border)
            .text_size(px(12.0))
            .child(section("Merge status", vec![merge], theme))
            .child(section("Checks", checks, theme))
            .child(section("Reviewers", reviews, theme))
            .child(section(
                "Assignees",
                pills(p.assignees.iter().map(String::as_str).collect()),
                theme,
            ))
            .child(section(
                "Labels",
                pills(p.labels.iter().map(String::as_str).collect()),
                theme,
            ))
            .child(section(
                "Milestone",
                p.milestone
                    .iter()
                    .map(|m| div().child(m.clone()).into_any_element())
                    .collect(),
                theme,
            ))
            .child(section("Changes", changes.into_iter().collect(), theme))
            .child(section("Participants", pills(participants), theme))
    }
}

/// A muted title over its rows, or "None" in faint text when there are none.
fn section(title: &'static str, rows: Vec<AnyElement>, theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.text_muted)
                .child(title),
        )
        .when(rows.is_empty(), |d| {
            d.child(div().text_color(theme.text_faint).child("None"))
        })
        .children(rows)
}
