//! How a status looks: label, glyph and color token, as a round icon and as a labelled pill.

use gd_forge::Status;
use gd_ui::{Icon, Theme, icon};
use gpui::{ColorExt, Div, Hsla, Styled, div, prelude::*, px};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Success,
    Danger,
    Warning,
    Running,
    Muted,
}

impl Tone {
    pub fn color(self, theme: &Theme) -> Hsla {
        match self {
            Self::Success => theme.success,
            Self::Danger => theme.danger,
            Self::Warning => theme.warning,
            Self::Running => theme.accent,
            Self::Muted => theme.text_muted,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Look {
    pub label: &'static str,
    pub icon: Icon,
    pub tone: Tone,
}

/// `warnings`: the pipeline passed but a job that may fail did not.
pub fn look(status: Status, warnings: bool) -> Look {
    let (label, icon, tone) = match status {
        Status::Success if warnings => ("warning", Icon::Warning, Tone::Warning),
        Status::Success => ("passed", Icon::Check, Tone::Success),
        Status::Failed => ("failed", Icon::Close, Tone::Danger),
        Status::Running => ("running", Icon::Play, Tone::Running),
        Status::Pending => ("pending", Icon::Clock, Tone::Warning),
        Status::WaitingForResource => ("waiting", Icon::Clock, Tone::Warning),
        Status::Preparing => ("preparing", Icon::Clock, Tone::Warning),
        Status::Scheduled => ("scheduled", Icon::Clock, Tone::Muted),
        Status::Created => ("created", Icon::Clock, Tone::Muted),
        Status::Manual => ("manual", Icon::Play, Tone::Muted),
        Status::Canceled => ("canceled", Icon::Stop, Tone::Muted),
        Status::Skipped => ("skipped", Icon::ArrowRight, Tone::Muted),
    };
    Look { label, icon, tone }
}

/// A ring with the status glyph in it. Static: a running status doesn't spin.
pub fn status_icon(look: Look, size: f32, theme: &Theme) -> Div {
    let color = look.tone.color(theme);
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(size))
        .rounded_full()
        .border_1()
        .border_color(color)
        .bg(color.opacity(0.12))
        .child(icon(look.icon).size(px(size * 0.6)).text_color(color))
}

/// The pill of a pipeline: icon and label.
pub fn status_badge(status: Status, warnings: bool, theme: &Theme) -> Div {
    let look = look(status, warnings);
    let color = look.tone.color(theme);
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.0))
        .h(px(22.0))
        .pl(px(3.0))
        .pr(px(9.0))
        .rounded_full()
        .bg(color.opacity(0.12))
        .text_size(px(12.0))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(color)
        .child(status_icon(look, 16.0, theme))
        .child(look.label)
}

#[cfg(test)]
#[path = "tests/status_badge.test.rs"]
mod tests;
