//! The job log: virtualized monospace lines with numbers, ANSI colors and collapsible sections.

use std::collections::HashSet;
use std::sync::Arc;

use gd_forge::{Log, Style};
use gd_ui::{Button, Icon, Theme, empty_state, icon};
use gpui::{
    Animation, AnimationExt, App, ClipboardItem, Entity, FontStyle, FontWeight, HighlightStyle,
    InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce, ScrollStrategy,
    SharedString, StatefulInteractiveElement, Styled, StyledText, UnderlineStyle, Window,
    combine_highlights, div, prelude::*, px, uniform_list,
};

use super::ansi::ansi_color;
use super::duration::format_duration;
use super::state::{CiStore, LOG_ANIM};
use crate::features::diff::text_selection::model::{TextPos, TextSel, TextSide};
use crate::workspace::Workspace;

const LINE_HEIGHT: f32 = 20.0;

/// Cubic ease-out: fast at first, settling softly.
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

#[derive(IntoElement)]
pub struct LogView {
    pub store: Entity<CiStore>,
    pub ws: Entity<Workspace>,
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
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .child(format!("Log unavailable: {title}")),
                    )
                    .child(div().text_color(theme.text_muted).child(text))
                    .child(
                        Button::new("ci-log-retry")
                            .icon(Icon::Refresh)
                            .label("Retry")
                            .on_click(move |_, _, cx| {
                                store.update(cx, |s, cx| s.load_log(job, cx))
                            }),
                    )
                    .into_any_element()
            }
            (None, None) => empty_state("Loading log…", cx).flex_1().into_any_element(),
            (Some(log), _) if log.lines.is_empty() => {
                empty_state("No output yet", cx).flex_1().into_any_element()
            }
            (Some(log), _) => self.lines(log.clone(), cx).into_any_element(),
        };
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(self.toolbar(cx))
            .child(body)
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
            .child(
                div()
                    .flex_1()
                    .text_size(px(12.0))
                    .text_color(theme.text_faint)
                    .child(format!("{count} lines")),
            )
            .child(
                Button::new("ci-log-top")
                    .ghost()
                    .compact()
                    .icon(Icon::ArrowUp)
                    .label("Top")
                    .on_click(move |_, _, cx| {
                        top.update(cx, |s, cx| {
                            s.log.follow = false;
                            s.log.scroll.scroll_to_item(0, ScrollStrategy::Top);
                            cx.notify()
                        })
                    }),
            )
            .child(
                Button::new("ci-log-bottom")
                    .ghost()
                    .compact()
                    .icon(Icon::ArrowDown)
                    .label("Bottom")
                    .on_click(move |_, _, cx| {
                        bottom.update(cx, |s, cx| {
                            s.log.follow = true;
                            s.log
                                .scroll
                                .scroll_to_item(s.log.rows() - 1, ScrollStrategy::Bottom);
                            cx.notify()
                        })
                    }),
            )
            .child(
                Button::new("ci-log-copy")
                    .ghost()
                    .compact()
                    .icon(Icon::Copy)
                    .label("Copy log")
                    .on_click(move |_, _, cx| {
                        if let Some(log) = &copy.read(cx).log.log {
                            let text = log
                                .lines
                                .iter()
                                .map(|l| l.text.as_str())
                                .collect::<Vec<_>>()
                                .join("\n");
                            cx.write_to_clipboard(ClipboardItem::new_string(text));
                        }
                    }),
            )
    }

    fn lines(&self, log: Arc<Log>, cx: &App) -> impl IntoElement + use<> {
        let ci = self.store.read(cx);
        let (store, ws, scroll, count) = (
            self.store.clone(),
            self.ws.clone(),
            ci.log.scroll.clone(),
            ci.log.rows(),
        );
        uniform_list("ci-log", count, move |range, _, cx| {
            let ci = store.read(cx);
            let theme = Theme::of(cx);
            let anim = ci
                .log
                .anim
                .as_ref()
                .filter(|a| a.at.elapsed() < LOG_ANIM * 2)
                .and_then(|a| Some((a, log.sections.get(a.section)?)));
            range
                .map(|row| match ci.log.visible.get(row) {
                    Some(&line) => {
                        let el =
                            log_line(line, &log, &ci.log.toggled, ci.log.sel, &store, &ws, theme);
                        let Some((a, s)) = anim else {
                            return el.into_any_element();
                        };
                        if line < s.header {
                            return el.into_any_element();
                        }
                        let (opening, rows_h, id) =
                            (a.opening, a.body.len() as f32 * LINE_HEIGHT, a.id);
                        let animation = Animation::new(LOG_ANIM).with_easing(ease_out_cubic);
                        if line == s.header {
                            // The rows of the section are in a container under its header, clipped to a height that grows or shrinks.
                            let rows = a.body.iter().map(|&l| {
                                log_line(l, &log, &ci.log.toggled, ci.log.sel, &store, &ws, theme)
                                    .into_any_element()
                            });
                            div()
                                .relative()
                                .w_full()
                                .child(el)
                                .child(
                                    div()
                                        .absolute()
                                        .top(px(LINE_HEIGHT))
                                        .left_0()
                                        .w_full()
                                        .overflow_hidden()
                                        .children(rows)
                                        .with_animation(
                                            SharedString::from(format!("ci-log-rows-{id}")),
                                            animation,
                                            move |el, t| {
                                                el.h(px(if opening { t } else { 1.0 - t } * rows_h))
                                            },
                                        ),
                                )
                                .into_any_element()
                        } else {
                            // The lines below are pushed by that container. Rows of a `uniform_list` are layout roots, which ignore
                            // a relative offset: it goes on a child.
                            div()
                                .w_full()
                                .flex()
                                .flex_col()
                                .child(div().w_full().child(el).with_animation(
                                    SharedString::from(format!("ci-log-below-{id}-{line}")),
                                    animation,
                                    move |el, t| {
                                        el.relative()
                                            .top(px(if opening { t } else { 1.0 - t } * rows_h))
                                    },
                                ))
                                .into_any_element()
                        }
                    }
                    None => div().h(px(LINE_HEIGHT)).into_any_element(),
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
fn log_line(
    line: usize,
    log: &Log,
    toggled: &HashSet<usize>,
    sel: Option<TextSel>,
    store: &Entity<CiStore>,
    ws: &Entity<Workspace>,
    theme: &Theme,
) -> impl IntoElement + use<> {
    let text = &log.lines[line];
    let section = log.sections.iter().position(|s| s.header == line);
    let ansi = text
        .spans
        .iter()
        .map(|span| (span.range.clone(), highlight(&span.style, theme)));
    let selected = sel.and_then(|s| s.cols_in(line, TextSide::Unified, text.text.len()));
    let wash = selected.map(|r| {
        (
            r,
            HighlightStyle {
                background_color: Some(theme.selection),
                ..Default::default()
            },
        )
    });
    let styled = StyledText::new(text.text.clone()).with_highlights(combine_highlights(ansi, wash));
    let layout = styled.layout().clone();
    // While dragging, the pointer over any part of a row moves the caret to the nearest position in its text.
    let drag = {
        let (store, hit) = (store.clone(), layout.clone());
        move |e: &gpui::MouseMoveEvent, _: &mut Window, cx: &mut App| {
            if e.pressed_button == Some(MouseButton::Left) && store.read(cx).log.dragging {
                let pos = TextPos {
                    line,
                    col: hit
                        .index_for_position(e.position)
                        .unwrap_or_else(|nearest| nearest),
                };
                store.update(cx, |s, cx| s.log_drag(pos, cx));
            }
        }
    };
    let row = div()
        .id(("ci-log-line", line))
        .flex()
        .items_center()
        .h(px(LINE_HEIGHT))
        .w_full()
        .overflow_hidden()
        .whitespace_nowrap()
        .on_mouse_move(drag);
    let number = div()
        .flex_none()
        .w(px(56.0))
        .pr(px(10.0))
        .text_right()
        .text_color(theme.text_faint)
        .child((line + 1).to_string());
    // The text sits in a flex column so it is measured and can hit-test (see `diff::highlight::line_text`).
    let cell = div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(styled);
    // The rows of an open section have the band of its header, to tell them from the rest.
    let inside = log.sections.iter().any(|s| s.header < line && line < s.end);
    let Some(ix) = section else {
        let (store, ws) = (store.clone(), ws.clone());
        let cell = cell
            .cursor_text()
            .on_mouse_down(MouseButton::Left, move |e, window, cx| {
                let pos = TextPos {
                    line,
                    col: layout
                        .index_for_position(e.position)
                        .unwrap_or_else(|nearest| nearest),
                };
                // Take focus away from anything else so Ctrl/Cmd+C and +A reach the log.
                ws.update(cx, |w, cx| w.focus(window, cx));
                store.update(cx, |s, cx| {
                    s.log_press(pos, e.click_count, e.modifiers.shift, cx)
                });
            });
        return row
            .when(inside, |d| d.bg(theme.ink(0.05)))
            .child(number)
            .child(div().flex_none().w(px(18.0)))
            .child(cell)
            .into_any_element();
    };
    let s = &log.sections[ix];
    let collapsed = s.collapsed != toggled.contains(&ix);
    let store = store.clone();
    // A top-level section is a step (GitLab script stage, GitHub/Gitea group): its header is a band that tells the steps apart.
    let (rest, hover) = if s.depth == 0 || inside {
        (theme.ink(0.05), theme.ink(0.09))
    } else {
        (theme.ink(0.0), theme.element_hover)
    };
    row.cursor_pointer()
        .bg(rest)
        .when(s.depth == 0, |d| d.border_t_1().border_color(theme.border))
        .hover_style(move |d| d.bg(hover))
        .on_click(move |_, _, cx| store.update(cx, |s, cx| s.toggle_section(ix, cx)))
        .child(number)
        .child(
            div().flex_none().w(px(18.0)).child(
                icon(if collapsed {
                    Icon::ChevronRight
                } else {
                    Icon::ChevronDown
                })
                .size(px(12.0))
                .text_color(theme.text_muted),
            ),
        )
        .child(cell)
        .children(s.duration.map(|d| {
            div()
                .flex_none()
                .px(px(12.0))
                .text_color(theme.text_faint)
                .child(format_duration(d))
        }))
        .into_any_element()
}

/// Bold is ignored: the mono font has no bold face, and the synthesized one garbles glyph widths.
fn highlight(style: &Style, theme: &Theme) -> HighlightStyle {
    let color = |c| ansi_color(c, theme.light);
    HighlightStyle {
        color: style.fg.map(color),
        background_color: style.bg.map(color),
        font_style: style.italic.then_some(FontStyle::Italic),
        underline: style.underline.then(|| UnderlineStyle {
            thickness: px(1.0),
            ..Default::default()
        }),
        ..Default::default()
    }
}
