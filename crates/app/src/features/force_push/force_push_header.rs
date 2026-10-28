//! Header of the force push preview: warning, what is sent and dropped,
//! the file change counts and the Cancel / Force push buttons.

use gd_git::{Commit, FileStatus};
use gd_ui::{Button, ButtonStyle, Icon, Theme, icon};
use gpui::{App, Entity, IntoElement, RenderOnce, SharedString, Window, div, prelude::*, px};

use crate::components::DiffStat;
use crate::repo::RepoModel;
use crate::utils::text::plural;
use crate::workspace::Workspace;

/// Commits listed per side before "+N more".
const COMMIT_ROWS: usize = 4;

#[derive(IntoElement)]
pub struct ForcePushHeader {
    pub repo: Entity<RepoModel>,
    pub ws: Entity<Workspace>,
}

impl RenderOnce for ForcePushHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let r = self.repo.read(cx);
        let preview = r.force_push.preview.as_ref();
        let remote = preview.map_or("the remote".to_string(), |p| p.remote_ref.clone());
        let ready = preview.is_some();
        let (cancel, confirm) = (self.ws.clone(), self.ws.clone());
        let title = div()
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(icon(Icon::ArrowUp).size(px(16.0)).text_color(theme.text))
            .child(
                div()
                    .flex_1()
                    .text_size(px(15.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(format!("Force push to {remote}")),
            )
            .child(
                Button::new("force-push-cancel")
                    .ghost()
                    .label("Cancel")
                    .on_click(move |_, w, cx| cancel.update(cx, |this, cx| this.close_overlay(w, cx))),
            )
            .child(
                Button::new("force-push-confirm")
                    .style(ButtonStyle::Danger)
                    .label("Force push")
                    .disabled(!ready)
                    .on_click(move |_, w, cx| confirm.update(cx, |this, cx| this.confirm_force_push(w, cx))),
            );
        let warning = r.force_push.error.clone().unwrap_or_else(|| {
            format!(
                "This replaces the history on {remote} with yours. Anyone who already pulled the removed commits will have to \
                 reconcile their work. The push is refused if the remote changed since your last fetch."
            )
        });
        div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(10.0))
            .px(px(16.0))
            .py(px(12.0))
            .border_b_1()
            .border_color(theme.border)
            .child(title)
            .child(
                div()
                    .flex()
                    .gap(px(6.0))
                    .text_size(px(12.0))
                    .text_color(theme.warning)
                    .child(icon(Icon::Info).size(px(13.0)).text_color(theme.warning))
                    .child(div().flex_1().min_w_0().child(warning)),
            )
            .children(preview.map(|p| {
                div()
                    .flex()
                    .gap(px(24.0))
                    .child(commit_column("Will be pushed".into(), &p.outgoing, &theme))
                    .child(commit_column(format!("Will be removed from {remote}"), &p.replaced, &theme))
            }))
            .children(preview.map(|p| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .text_size(px(12.5))
                    .text_color(theme.text_muted)
                    .child(change_summary(p.files.iter().map(|f| f.status)))
                    .child(DiffStat { added: p.stats.added, deleted: p.stats.deleted })
            }))
    }
}

fn commit_column(title: String, commits: &[Commit], theme: &Theme) -> impl IntoElement + use<> {
    let more = commits.len().saturating_sub(COMMIT_ROWS);
    let rows = commits.iter().take(COMMIT_ROWS).map(|c| {
        div()
            .flex()
            .gap(px(8.0))
            .text_size(px(12.5))
            .child(div().font_family("monospace").text_color(theme.text_faint).child(c.short_sha.clone()))
            .child(div().flex_1().min_w_0().truncate().text_color(theme.text).child(c.summary.clone()))
    });
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .gap(px(3.0))
        .child(
            div()
                .text_size(px(12.0))
                .text_color(theme.text_muted)
                .child(format!("{title} ({})", commits.len())),
        )
        .children(rows)
        .children((more > 0).then(|| div().text_size(px(12.0)).text_color(theme.text_faint).child(format!("+{more} more"))))
}

/// "2 added • 3 modified • 1 deleted • 1 renamed" for what the remote's files would go through.
fn change_summary(statuses: impl Iterator<Item = FileStatus>) -> SharedString {
    let mut counts = [0u32; 4];
    for s in statuses {
        let slot = match s {
            FileStatus::New | FileStatus::Untracked | FileStatus::Copied => 0,
            FileStatus::Deleted => 2,
            FileStatus::Renamed => 3,
            _ => 1,
        };
        counts[slot] += 1;
    }
    let parts: Vec<String> = ["added", "modified", "deleted", "renamed"]
        .iter()
        .zip(counts)
        .filter(|(_, n)| *n > 0)
        .map(|(label, n)| format!("{n} {label}"))
        .collect();
    match counts.iter().sum::<u32>() {
        0 => "No file changes".into(),
        n => format!("{n} file{} changed: {}", plural(n), parts.join(" • ")).into(),
    }
}
