//! Snapshot views: file selection, commit box, undo strip, sidebar width.

use gpui::{Context, Window};

use crate::snapshot::{Step, View, press_next};
use crate::workspace::Workspace;

const NEXT: Step = |ws, _, w, cx| press_next(None, ws, w, cx);

pub fn views() -> &'static [View] {
    &[
        View { name: "changes@*", steps: &[select_file] },
        View { name: "commit-long", steps: &[commit_long] },
        View { name: "undo-strip", steps: &[undo_strip] },
        View {
            name: "sidebar-wide",
            steps: &[|ws, _, _, cx| {
                ws.settings.sidebar_width = Some(420.0);
                cx.notify();
            }],
        },
        View { name: "nav-changes", steps: &[NEXT, NEXT, NEXT] },
    ]
}

/// `changes@N`: select the N-th changed file.
fn select_file(ws: &mut Workspace, view: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    let n = view["changes@".len()..].parse().unwrap_or(0);
    if let Some(r) = &ws.repo {
        r.update(cx, |r, cx| r.select_change(n, cx));
    }
}

/// Multi-line, wrapping message typed into the commit box.
fn commit_long(ws: &mut Workspace, _: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    let body = (1..=12).map(|i| format!("Line {i} of a long description that wraps around the box")).collect::<Vec<_>>().join("\n");
    let summary = "A fairly long summary supercalifragilisticexpialidocious_unbreakable_identifier_name_here_and_more";
    ws.commit_box.update(cx, |b, cx| b.set_text(summary, &body, cx));
}

fn undo_strip(ws: &mut Workspace, _: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    let Some(r) = &ws.repo else { return };
    r.update(cx, |r, cx| {
        r.last_commit = Some(crate::repo::LastCommit {
            summary: "Add module 5 with some changes".into(),
            description: "Body".into(),
            branch: r.branch_name(),
            time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
        });
        cx.notify();
    });
}
