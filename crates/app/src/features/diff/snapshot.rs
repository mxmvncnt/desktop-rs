//! Snapshot views: unified / split / wrapped / scrolled diffs, options menu, hunk expansion,
//! line selection and text selection. Options are set directly: no settings file is written.

use gpui::{Context, Window, point, px};

use super::DiffPane;
use super::text_selection::{TextPos, TextSel, TextSide};
use crate::repo::Tab;
use crate::snapshot::View;
use crate::workspace::Workspace;

pub fn views() -> &'static [View] {
    &[
        View { name: "split", steps: &[options] },
        View { name: "split-history", steps: &[options] },
        View { name: "hide-ws", steps: &[options] },
        View { name: "split-ws", steps: &[options] },
        View { name: "diff-options-menu", steps: &[options] },
        // Long lines: wrapped, or scrolled 160px right (both columns in split).
        View { name: "wrap", steps: &[long_lines] },
        View { name: "split-wrap", steps: &[long_lines] },
        View { name: "hscroll", steps: &[long_lines] },
        View { name: "split-hscroll", steps: &[long_lines] },
        View { name: "select*", steps: &[select_mode, select_text] },
        View { name: "partial*", steps: &[partial_file, partial_rows] },
        View { name: "expanded*", steps: &[expanded_mode, expand_hunks] },
    ]
}

fn options(ws: &mut Workspace, view: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.settings.diff_split = matches!(view, "split" | "split-history" | "split-ws" | "diff-options-menu");
    ws.settings.diff_ignore_whitespace = matches!(view, "hide-ws" | "split-ws");
    ws.apply_diff_options(cx);
    if view == "split-history" {
        if let Some(r) = &ws.repo {
            r.update(cx, |r, cx| r.set_tab(Tab::History, cx));
        }
    }
    if view == "diff-options-menu" {
        ws.open_diff_options(point(px(1240.0), px(60.0)), window, cx);
    }
    cx.notify();
}

fn long_lines(ws: &mut Workspace, view: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    ws.settings.diff_split = view.starts_with("split");
    ws.settings.diff_wrap = view.ends_with("wrap");
    if view.ends_with("hscroll") {
        ws.diff_view.hscroll = px(160.0);
    }
    cx.notify();
}

/// `select[-split][-wrap|-hscroll]`: text selection mode.
fn select_mode(ws: &mut Workspace, view: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    ws.settings.diff_split = view.contains("split");
    ws.settings.diff_wrap = view.contains("wrap");
    if view.contains("hscroll") {
        ws.diff_view.hscroll = px(160.0);
    }
    cx.notify();
}

/// ...then a preset selection across several lines.
fn select_text(ws: &mut Workspace, view: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    let Some(diff) = ws.diff_view.shown_diff() else { return };
    let side = if view.contains("split") { TextSide::Right } else { TextSide::Unified };
    let lines: Vec<usize> = (0..diff.lines.len())
        .filter(|&i| diff.lines[i].kind != gd_git::DiffLineKind::Hunk && !diff.lines[i].text.is_empty())
        .filter(|&i| side != TextSide::Right || diff.lines[i].kind != gd_git::DiffLineKind::Delete)
        .collect();
    let (Some(&a), Some(&b)) = (lines.get(3), lines.get(9)) else { return };
    let head = TextPos { line: b, col: diff.lines[b].text.len().min(14) };
    ws.diff_view.text_sel = Some(TextSel { anchor: TextPos { line: a, col: 6 }, head, side });
    cx.notify();
}

/// `partial[-split][@N]`: file N, ...
fn partial_file(ws: &mut Workspace, view: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    let n = view.split_once('@').and_then(|(_, n)| n.parse().ok()).unwrap_or(0);
    ws.settings.diff_split = view.starts_with("partial-split");
    ws.apply_diff_options(cx);
    if let Some(r) = &ws.repo {
        r.update(cx, |r, cx| r.select_change(n, cx));
    }
}

/// ...with some lines deselected.
fn partial_rows(ws: &mut Workspace, _: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    if let Some(r) = &ws.repo {
        r.update(cx, |r, cx| {
            let Some(diff) = r.work.detail.diff.clone() else { return };
            let rows: Vec<usize> = (0..diff.lines.len()).filter(|&i| gd_git::selectable(&diff, i)).collect();
            // Deselect the 2nd selectable row and the last change block.
            if let Some(&ix) = rows.get(1) {
                r.toggle_row(ix, cx);
            }
            if let Some(b) = rows.last().and_then(|&ix| crate::repo::RepoModel::change_block(&diff, ix)) {
                r.toggle_block(b, cx);
            }
        });
    }
    cx.notify();
}

fn expanded_mode(ws: &mut Workspace, view: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    ws.settings.diff_split = view == "expanded-split";
    ws.apply_diff_options(cx);
}

/// Hunk 0 expanded down once and the gap above the last hunk expanded fully (see `gd_git::expand`).
fn expand_hunks(ws: &mut Workspace, _: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    if let Some(r) = &ws.repo {
        r.update(cx, |r, cx| {
            let last = r.work.detail.diff.as_ref().map_or(0, |d| d.hunks.len().saturating_sub(1));
            r.expand_diff(DiffPane::Changes, gd_git::Expansion::Down(0), cx);
            r.expand_diff(DiffPane::Changes, gd_git::Expansion::All(last), cx);
        });
    }
}
