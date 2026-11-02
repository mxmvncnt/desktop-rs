//! One row of the sidebar stash list: title, branch, relative time.

use gd_git::Stash;
use gd_ui::{Icon, Theme, icon, pressable};
use gpui::{App, Entity, IntoElement, MouseButton, RenderOnce, Window, div, prelude::*, px};

use crate::repo::RepoModel;
use crate::utils::text::relative_time;
use crate::workspace::Workspace;

pub const STASH_ROW_HEIGHT: f32 = 46.0;

#[derive(IntoElement)]
pub struct StashRow {
    pub stash: Stash,
    pub selected: bool,
    /// Made on the checked-out branch.
    pub here: bool,
    pub repo: Entity<RepoModel>,
    pub ws: Entity<Workspace>,
}

impl RenderOnce for StashRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let (here, selected) = (self.here, self.selected);
        let (stash, menu_stash) = (self.stash.clone(), self.stash.clone());
        let (repo, ws) = (self.repo, self.ws);
        pressable(("stash", stash.index), selected, cx)
            .flex()
            .flex_col()
            .justify_center()
            .gap(px(3.0))
            .h(px(STASH_ROW_HEIGHT))
            .w_full()
            .px(px(10.0))
            .rounded(px(6.0))
            .child(
                div()
                    .id(("stash-title", stash.index))
                    .tooltip(gd_ui::tooltip(stash.title()))
                    .truncate()
                    .text_size(px(13.0))
                    .text_color(if here || selected { theme.text } else { theme.text_muted })
                    .child(stash.title()),
            )
            .child(meta_line(&stash, here, &theme))
            .on_mouse_down(MouseButton::Right, move |e, w, cx| {
                let (pos, s) = (e.position, menu_stash.clone());
                ws.update(cx, |this, cx| this.open_stash_menu(s, pos, w, cx))
            })
            .on_click(move |_, _, cx| {
                let sha = stash.sha.clone();
                repo.update(cx, |r, cx| r.select_stash(sha, cx))
            })
    }
}

/// Second line: branch name, relative time.
fn meta_line(s: &Stash, here: bool, theme: &Theme) -> impl IntoElement + use<> {
    div()
        .flex()
        .items_center()
        .gap(px(5.0))
        .text_size(px(11.5))
        .text_color(theme.text_faint)
        .child(icon(Icon::GitBranch).size(px(11.0)).text_color(if here { theme.accent } else { theme.text_faint }))
        .child(
            div()
                .flex_shrink(1.0)
                .min_w_0()
                .truncate()
                .text_color(if here { theme.text_muted } else { theme.text_faint })
                .child(s.branch.clone().unwrap_or_else(|| "(no branch)".into())),
        )
        .child(div().flex_none().child(format!("• {}", relative_time(s.time))))
}
