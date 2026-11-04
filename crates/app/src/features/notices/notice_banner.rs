use std::collections::hash_map::DefaultHasher;
use std::hash::{BuildHasher, BuildHasherDefault};

use gd_ui::{Icon, Theme, icon, motion};
use gpui::{
    App, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div, px,
};

use crate::components::{drag_selection, selectable_line};
use crate::features::diff::text_selection::model::TextSel;
use crate::workspace::Workspace;

#[derive(Clone, Copy, PartialEq, Eq)]
enum NoticeKind {
    Error,
    Success,
}

#[derive(IntoElement)]
pub struct NoticeBanner {
    ws: Entity<Workspace>,
    kind: NoticeKind,
    message: String,
    sel: Option<TextSel>,
}

impl NoticeBanner {
    /// `None` when there is nothing to show. Errors win over notices.
    pub fn open(ws: &Workspace, cx: &mut gpui::Context<Workspace>) -> Option<Self> {
        let (error, notice) = match &ws.repo {
            Some(r) => {
                let r = r.read(cx);
                (r.error.clone(), r.notice.clone())
            }
            None => (ws.startup_error.clone(), None),
        };
        let (kind, message) = match (error, notice) {
            (Some(e), _) => (NoticeKind::Error, e),
            (None, Some(n)) => (NoticeKind::Success, n),
            _ => return None,
        };
        let sel = ws.commit_sel.on(&[message.clone(), String::new()]);
        Some(Self {
            ws: cx.entity(),
            kind,
            message,
            sel,
        })
    }
}

impl RenderOnce for NoticeBanner {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let is_error = self.kind == NoticeKind::Error;
        let key = format!(
            "banner-{}",
            BuildHasherDefault::<DefaultHasher>::default().hash_one(&self.message)
        );
        let texts = [self.message, String::new()];
        let (message, layout) = selectable_line(
            &self.ws,
            &texts,
            0,
            self.sel,
            theme.text,
            FontWeight::NORMAL,
            &theme,
        );
        let ws = self.ws;
        div()
            .absolute()
            .bottom(px(16.0))
            .right(px(16.0))
            .max_w(px(520.0))
            .child(motion::fade_in(
                SharedString::from(key),
                gd_ui::overlay_card(cx)
                    .occlude()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .p(px(12.0))
                    .on_mouse_move(drag_selection(ws.clone(), vec![layout]))
                    .child(
                        icon(if is_error { Icon::Warning } else { Icon::Check }).text_color(
                            if is_error {
                                theme.danger
                            } else {
                                theme.success
                            },
                        ),
                    )
                    .child(message.flex_1().min_w_0().text_size(px(12.5)))
                    .child(
                        gd_ui::icon_button("dismiss-banner", Icon::Close)
                            .compact()
                            .on_click(move |_, _, cx| {
                                ws.update(cx, |this, cx| this.dismiss_notices(cx))
                            }),
                    ),
            ))
    }
}
