//! The Commits tab's list: the pull request's commits, oldest first; the selected one's diff is shown beside it.

use gd_git::Commit;
use gd_ui::{Theme, pressable};
use gpui::{
    App, Entity, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
    Window, div, px,
};

use super::state::PullsStore;
use crate::components::list;
use crate::utils::text::relative_time;

const ROW: f32 = 48.0;

#[derive(IntoElement)]
pub struct CommitsList {
    pub store: Entity<PullsStore>,
    pub commits: Vec<Commit>,
}

impl RenderOnce for CommitsList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let pulls = self.store.read(cx);
        let (selected, scroll) = (pulls.commit, pulls.commits_scroll.clone());
        let (store, commits) = (self.store, self.commits);
        list("pull-commits", commits.len(), move |range, _, cx| {
            let theme = Theme::of(cx);
            range
                .map(|ix| {
                    let c = &commits[ix];
                    let store = store.clone();
                    pressable(("pull-commit", ix), selected == Some(ix), cx)
                        .flex()
                        .flex_col()
                        .justify_center()
                        .gap(px(2.0))
                        .h(px(ROW))
                        .w_full()
                        .px(px(12.0))
                        .rounded(px(6.0))
                        .on_click(move |_, _, cx| store.update(cx, |s, cx| s.select_commit(ix, cx)))
                        .child(div().truncate().child(c.summary.clone()))
                        .child(
                            div()
                                .flex()
                                .gap(px(6.0))
                                .text_size(px(11.5))
                                .text_color(theme.text_muted)
                                .child(
                                    div()
                                        .font_family(theme.font_mono.clone())
                                        .child(c.short_sha.clone()),
                                )
                                .child(div().truncate().child(format!(
                                    "{} • {}",
                                    c.author_name,
                                    relative_time(c.time)
                                ))),
                        )
                        .into_any_element()
                })
                .collect()
        })
        .track_scroll(&scroll)
        .flex_1()
    }
}
