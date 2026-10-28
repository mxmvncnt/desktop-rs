//! Desktop's "Committed just now" strip with an Undo button.

use gd_ui::{Button, Icon, Theme, icon, icon_button, motion};
use gpui::{
    App, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px,
};

use crate::repo::LastCommit;
use crate::utils::text::relative_time;
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct UndoStrip {
    pub commit: LastCommit,
    pub ws: Entity<Workspace>,
}

impl RenderOnce for UndoStrip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let c = self.commit;
        let ws = self.ws;
        let dismiss = ws.clone();
        motion::fade_quick(
            ("undo-strip", c.time as usize),
            div()
                .group("undo-strip")
                .flex()
                .flex_none()
                .items_center()
                .gap(px(10.0))
                .px(px(12.0))
                .py(px(8.0))
                .border_t_1()
                .border_color(theme.border)
                .bg(theme.ink(0.03))
                .child(
                    div()
                        .relative()
                        .flex_none()
                        .size(px(22.0))
                        .child(
                            div()
                                .absolute()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .group_hover_style("undo-strip", |s| s.invisible())
                                .child(icon(Icon::Check).size(px(14.0)).text_color(theme.success)),
                        )
                        .child(
                            div().absolute().size_full().invisible().group_hover_style("undo-strip", |s| s.visible()).child(
                                icon_button("dismiss-undo-strip", Icon::Close).compact().on_click(move |_, _, cx| {
                                    dismiss.update(cx, |this, cx| this.dismiss_undo_strip(cx))
                                }),
                            ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(theme.text_faint)
                                .child(format!("Committed {}", relative_time(c.time))),
                        )
                        .child(div().text_size(px(12.5)).truncate().child(c.summary)),
                )
                .child(
                    Button::new("undo-commit")
                        .compact()
                        .icon(Icon::Restart)
                        .label("Undo")
                        .on_click(move |_, _, cx| ws.update(cx, |this, cx| this.undo_commit(cx))),
                ),
        )
    }
}
