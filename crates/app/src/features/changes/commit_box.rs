//! Summary + description inputs and the commit button.

use gd_ui::{Button, Icon, Theme};
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled, Window, div, px};
use gpui_elements::editable_text::EditableTextState;

use super::amend_notice::AmendNotice;
use super::state::CommitButton;
use crate::components::text_area;
use crate::workspace::Workspace;

const PADDING: f32 = 12.0;
/// The message input grows up to this height, then scrolls.
const MAX_INPUT_HEIGHT: f32 = 200.0;
const SIDEBAR_BORDER: f32 = 1.0;

#[derive(IntoElement)]
pub struct CommitBox {
    pub button: CommitButton,
    pub message: Entity<EditableTextState>,
    /// Sidebar width, to give the input a definite width (see `message_input`).
    pub sidebar_width: f32,
    pub ws: Entity<Workspace>,
}

impl CommitBox {

    fn message_input(&self, cx: &App) -> impl IntoElement {
        let placeholder = self.button.default_summary.as_deref().unwrap_or("Commit message (required)");
        text_area("message", &self.message, placeholder, cx)
            // A definite width: with `w_full` gpui-ce also measures the text at max-content
            // width, and its layout cache then paints that unwrapped layout.
            .w(px(self.sidebar_width - SIDEBAR_BORDER - 2.0 * PADDING))
            .min_h(px(64.0))
            .max_h(px(MAX_INPUT_HEIGHT))
            .flex_col()
            .whitespace_normal()
            .overflow_y_scroll()
    }
}

impl RenderOnce for CommitBox {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let b = &self.button;
        let amend_notice = b.amending.then(|| AmendNotice { pushed: b.head_pushed, ws: self.ws.clone() });
        let commit = Button::new("commit")
            .primary()
            .full_width()
            .icon(if b.amending { Icon::Pen } else { Icon::Check })
            .label(b.label.clone())
            .disabled(!b.enabled)
            .on_click(|_, w, cx| w.dispatch_action(Box::new(crate::actions::Commit), cx));
        div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(8.0))
            .p(px(PADDING))
            .border_t_1()
            .border_color(theme.border)
            .children(amend_notice)
            .child(self.message_input(cx))
            .child(commit)
    }
}
