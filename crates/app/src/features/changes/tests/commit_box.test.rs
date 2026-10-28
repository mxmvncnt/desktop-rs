//! Typing in the commit box must notify the `CommitBox` view only, never `Workspace`.
//! Needs gpui's test platform, hence the `snapshot` feature (`cargo test --features snapshot`).

use std::cell::Cell;
use std::rc::Rc;

use gpui::{Entity, TestAppContext};

use super::*;
use crate::workspace::Workspace;

#[gpui::test]
fn typing_does_not_notify_workspace(cx: &mut TestAppContext) {
    // Not a tempdir: deleting the repo at the end would wake the file watcher's thread, which
    // the test scheduler rejects. A fixed scratch dir is wiped at the start instead.
    let dir = std::path::Path::new("/tmp/desktop-rs/commit-box-test");
    std::fs::remove_dir_all(dir).ok();
    let repo = dir.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    assert!(std::process::Command::new("git").args(["init", "-q"]).current_dir(&repo).status().unwrap().success());
    // SAFETY: the only test touching the environment; settings are written here, not to ~/.config.
    unsafe { std::env::set_var("GD_CONFIG_DIR", dir.join("config")) };

    cx.update(|cx| {
        gd_ui::init(cx);
        crate::actions::bind(cx);
    });
    let (ws, cx) = cx.add_window_view(|window, cx| Workspace::new(Some(repo), window, cx));
    cx.run_until_parked();

    let workspace_notifies = Rc::new(Cell::new(0));
    let commit_box_notifies = Rc::new(Cell::new(0));
    let commit_box: Entity<CommitBox> = ws.read_with(cx, |ws, _| ws.commit_box.clone());
    let _subs = cx.update(|window, cx| {
        window.focus(&gpui::Focusable::focus_handle(&commit_box.read(cx).message, cx), cx);
        let (w, c) = (workspace_notifies.clone(), commit_box_notifies.clone());
        [
            cx.observe(&ws, move |_, _| w.set(w.get() + 1)),
            cx.observe(&commit_box, move |_, _| c.set(c.get() + 1)),
        ]
    });
    cx.run_until_parked();

    cx.simulate_input("hello");
    cx.run_until_parked();

    assert_eq!(commit_box.read_with(cx, |b, cx| b.text(cx)), ("hello".to_string(), String::new()));
    assert!(commit_box_notifies.get() > 0, "the commit box re-renders on input");
    assert_eq!(workspace_notifies.get(), 0, "typing must not notify Workspace");
}
