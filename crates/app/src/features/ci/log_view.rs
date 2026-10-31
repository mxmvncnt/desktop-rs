//! The job log: virtualized monospace lines with numbers, ANSI colors and collapsible sections.

use std::collections::HashSet;
use std::sync::Arc;

use gd_forge::{Log, Style};
use gd_ui::{Button, Icon, Theme, empty_state, icon};
use gpui::{
    App, ClipboardItem, Entity, FontStyle, FontWeight, HighlightStyle, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    ScrollStrategy, StatefulInteractiveElement, Styled, StyledText, UnderlineStyle, Window, div, px, uniform_list,
};

use super::ansi::ansi_color;
use super::duration::format_duration;
use super::state::CiStore;

const LINE_HEIGHT: f32 = 20.0;

#[derive(IntoElement)]
pub struct LogView {
    pub store: Entity<CiStore>,
    pub job: gd_forge::JobId,
}

impl RenderOnce for LogView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let ci = self.store.read(cx);
        let state = &ci.log;
        let body = match (&state.log, &state.error) {
            (None, Some(problem)) => {
                let (title, text) = problem.describe();
                let store = self.store.clone();
                let job = self.job;
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(8.0))
                    .child(div().font_weight(FontWeight::MEDIUM).child(format!("Log unavailable: {title}")))
                    .child(div().text_color(theme.text_muted).child(text))
                    .child(Button::new("ci-log-retry").icon(Icon::Refresh).label("Retry").on_click(move |_, _, cx| store.update(cx, |s, cx| s.load_log(job, cx))))
                    .into_any_element()
            }
            (None, None) => empty_state("Loading log…", cx).flex_1().into_any_element(),
            (Some(log), _) if log.lines.is_empty() => empty_state("No output yet", cx).flex_1().into_any_element(),
            (Some(log), _) => self.lines(log.clone(), cx).into_any_element(),
        };
        div().flex_1().min_w_0().flex().flex_col().child(self.toolbar(cx)).child(body)
    }
}

impl LogView {
    fn toolbar(&self, cx: &App) -> impl IntoElement + use<> {
        let theme = Theme::of(cx);
        let state = &self.store.read(cx).log;
        let (top, bottom, copy) = (self.store.clone(), self.store.clone(), self.store.clone());
        let count = state.log.as_ref().map_or(0, |l| l.lines.len());
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(6.0))
            .h(px(40.0))
            .px(px(12.0))
            .border_b_1()
            .border_color(theme.border)
            .child(div().flex_1().text_size(px(12.0)).text_color(theme.text_faint).child(format!("{count} lines")))
            .child(Button::new("ci-log-top").ghost().compact().icon(Icon::ArrowUp).label("Top").on_click(move |_, _, cx| {
                top.update(cx, |s, cx| {
                    s.log.follow = false;
                    s.log.scroll.scroll_to_item(0, ScrollStrategy::Top);
                    cx.notify()
                })
            }))
            .child(Button::new("ci-log-bottom").ghost().compact().icon(Icon::ArrowDown).label("Bottom").on_click(move |_, _, cx| {
                bottom.update(cx, |s, cx| {
                    s.log.follow = true;
                    s.log.scroll.scroll_to_item(s.log.visible.len().saturating_sub(1), ScrollStrategy::Bottom);
                    cx.notify()
                })
            }))
            .child(Button::new("ci-log-copy").ghost().compact().icon(Icon::Copy).label("Copy log").on_click(move |_, _, cx| {
                if let Some(log) = &copy.read(cx).log.log {
                    let text = log.lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n");
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }))
    }

    fn lines(&self, log: Arc<Log>, cx: &App) -> impl IntoElement + use<> {
        let ci = self.store.read(cx);
        let (store, scroll, count) = (self.store.clone(), ci.log.scroll.clone(), ci.log.visible.len());
        uniform_list("ci-log", count, move |range, _, cx| {
            let ci = store.read(cx);
            let theme = Theme::of(cx);
            range
                .filter_map(|row| {
                    let line = *ci.log.visible.get(row)?;
                    Some(log_line(line, &log, &ci.log.toggled, &store, theme))
                })
                .collect()
        })
        .track_scroll(&scroll)
        .flex_1()
        .font_family(Theme::of(cx).font_mono.clone())
        .text_size(px(12.0))
    }
}

/// A plain fn: rows of a virtualized list are rebuilt every frame.
fn log_line(line: usize, log: &Log, toggled: &HashSet<usize>, store: &Entity<CiStore>, theme: &Theme) -> impl IntoElement + use<> {
    let text = &log.lines[line];
    let section = log.sections.iter().position(|s| s.header == line);
    let highlights = text.spans.iter().map(|span| (span.range.clone(), highlight(&span.style, theme)));
    let styled = StyledText::new(text.text.clone()).with_highlights(highlights);
    let row = div().id(("ci-log-line", line)).flex().items_center().h(px(LINE_HEIGHT)).w_full().overflow_hidden().whitespace_nowrap();
    let number = div().flex_none().w(px(56.0)).pr(px(10.0)).text_right().text_color(theme.text_faint).child((line + 1).to_string());
    let Some(ix) = section else {
        return row.child(number).child(div().flex_none().w(px(18.0))).child(div().flex_1().min_w_0().overflow_hidden().child(styled)).into_any_element();
    };
    let s = &log.sections[ix];
    let collapsed = s.collapsed != toggled.contains(&ix);
    let store = store.clone();
    row.cursor_pointer()
        .hover_style(|d| d.bg(theme.element_hover))
        .on_click(move |_, _, cx| store.update(cx, |s, cx| s.toggle_section(ix, cx)))
        .child(number)
        .child(div().flex_none().w(px(18.0)).child(icon(if collapsed { Icon::ChevronRight } else { Icon::ChevronDown }).size(px(12.0)).text_color(theme.text_muted)))
        .child(div().flex_1().min_w_0().overflow_hidden().child(styled))
        .children(s.duration.map(|d| div().flex_none().px(px(12.0)).text_color(theme.text_faint).child(format_duration(d))))
        .into_any_element()
}

/// Bold is ignored: the mono font has no bold face, and the synthesized one garbles glyph widths.
fn highlight(style: &Style, theme: &Theme) -> HighlightStyle {
    let color = |c| ansi_color(c, theme.light);
    HighlightStyle {
        color: style.fg.map(color),
        background_color: style.bg.map(color),
        font_style: style.italic.then_some(FontStyle::Italic),
        underline: style.underline.then(|| UnderlineStyle { thickness: px(1.0), ..Default::default() }),
        ..Default::default()
    }
}
