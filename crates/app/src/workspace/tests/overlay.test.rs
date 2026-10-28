//! Key paths of the overlay stack through a real `Workspace` and GPUI's test platform (needs
//! `--features snapshot`): keystrokes go through `Window::dispatch_keystroke`, so the bindings in
//! `actions::bind`, the key contexts and the dialogs' own Enter/Esc capture are all exercised.

use std::cell::RefCell;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;
use std::sync::{Arc, Once};
use std::time::{Duration, Instant};

use gd_git::Commit;
use gd_ui::Icon;
use gpui::{
    AppContext as _, ClickEvent, Focusable, HeadlessAppContext, Keystroke, MouseButton, MouseClickEvent,
    MouseDownEvent, MouseUpEvent, PlatformInput, Window, WindowHandle, point, px, size,
};

use super::*;
use crate::features::branches::BranchDialog;
use crate::features::merge::MergeMode;
use crate::components::MenuRow;
use crate::features::history::HistoryDialog;
use crate::features::stashes::StashDialog;
use crate::platform::SECONDARY;

struct Fixture {
    cx: HeadlessAppContext,
    window: WindowHandle<Workspace>,
    _dir: tempfile::TempDir,
}

fn git(repo: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(["-c", "user.name=T", "-c", "user.email=t@example.com"])
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

/// A repo on `main` with a `feature` branch one commit ahead, opened in a focused workspace.
fn fixture() -> Fixture {
    static CONFIG: Once = Once::new();
    let config = tempfile::tempdir().unwrap();
    // SAFETY: set once before any workspace exists, so settings never touch the real config dir.
    CONFIG.call_once(|| unsafe { std::env::set_var("GD_CONFIG_DIR", config.keep()) });

    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    git(repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("a.txt"), "a\n").unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-qm", "init"]);
    git(repo, &["checkout", "-qb", "feature"]);
    git(repo, &["commit", "-q", "--allow-empty", "-m", "feature work"]);
    git(repo, &["checkout", "-q", "main"]);

    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(gd_ui::Assets), || None);
    cx.update(|cx| {
        gd_ui::init(cx);
        crate::actions::bind(cx);
        cx.bind_keys(gpui_elements::editable_text::actions::default_bindings().as_keybindings(Some(
            gpui_elements::editable_text::actions::DEFAULT_INPUT_CONTEXT,
        )));
    });
    let window = cx
        .open_window(size(px(1280.0), px(820.0)), |window, cx| cx.new(|cx| Workspace::new(Some(repo.to_path_buf()), window, cx)))
        .unwrap();
    let mut f = Fixture { cx, window, _dir: dir };
    f.wait(|f| f.read(|ws, cx| ws.repo.as_ref().is_some_and(|r| r.read(cx).branches.len() == 2)));
    f.with(|ws, w, cx| ws.focus(w, cx));
    f
}

impl Fixture {
    /// Run `f` on the workspace with the window, then let spawned tasks and the next frame run.
    fn with<R>(&mut self, f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) -> R) -> R {
        let ws = self.cx.update_window(self.window.into(), |root, _, _| root.downcast::<Workspace>().unwrap()).unwrap();
        let r = self.cx.update_window(self.window.into(), |_, w, cx| ws.update(cx, |ws, cx| f(ws, w, cx))).unwrap();
        self.cx.run_until_parked();
        r
    }

    fn read<R>(&mut self, f: impl FnOnce(&Workspace, &gpui::App) -> R) -> R {
        let ws = self.cx.update_window(self.window.into(), |root, _, _| root.downcast::<Workspace>().unwrap()).unwrap();
        self.cx.update(|cx| f(ws.read(cx), cx))
    }

    /// Press keys like the user would, e.g. `press("down")`, `press("ctrl-b")`.
    fn press(&mut self, key: &str) {
        let key = Keystroke::parse(key).unwrap();
        self.cx.update_window(self.window.into(), |_, w, cx| w.dispatch_keystroke(key, cx)).unwrap();
        self.cx.run_until_parked();
    }

    /// A full left click at `(x, y)`.
    fn click(&mut self, x: f32, y: f32) {
        let position = point(px(x), px(y));
        let (button, modifiers) = (MouseButton::Left, Default::default());
        let down = MouseDownEvent { button, position, modifiers, click_count: 1, first_mouse: false };
        let up = MouseUpEvent { button, position, modifiers, click_count: 1 };
        for ev in [PlatformInput::MouseDown(down), PlatformInput::MouseUp(up)] {
            self.cx.update_window(self.window.into(), |_, w, cx| w.dispatch_event(ev, cx)).unwrap();
        }
        self.cx.run_until_parked();
    }

    /// Wait (real time: git runs on background threads) until `done`.
    fn wait(&mut self, done: impl Fn(&mut Self) -> bool) {
        let end = Instant::now() + Duration::from_secs(10);
        while !done(self) {
            assert!(Instant::now() < end, "timed out");
            std::thread::sleep(Duration::from_millis(20));
            self.cx.run_until_parked();
        }
    }

    /// Open overlays, bottom first.
    fn stack(&mut self) -> Vec<&'static str> {
        self.read(|ws, _| {
            ws.overlays
                .iter()
                .map(|o| match o {
                    Overlay::Popover(_) => "popover",
                    Overlay::Menu(_) => "menu",
                    Overlay::DiffOptions(_) => "diff options",
                    Overlay::Settings => "settings",
                    Overlay::Discard(_) => "discard",
                    Overlay::History(_) => "history",
                    Overlay::Stash(_) => "stash",
                    Overlay::Merge(_) => "merge",
                    Overlay::Branch(_) => "branch",
                })
                .collect()
        })
    }

    fn open_menu(&mut self, rows: Vec<MenuRow<i32>>) -> Rc<RefCell<Vec<i32>>> {
        let ran = Rc::new(RefCell::new(Vec::new()));
        let log = ran.clone();
        let menu = Menu::new(point(px(100.0), px(100.0)), rows, move |_, a, _, _| log.borrow_mut().push(a));
        self.with(|ws, w, cx| ws.open_menu(menu, w, cx));
        ran
    }

    fn head_commit(&mut self) -> Commit {
        self.read(|ws, cx| ws.repo.as_ref().unwrap().read(cx).repo.log(0, 1).unwrap().remove(0))
    }
}

fn item(action: i32) -> MenuRow<i32> {
    MenuRow::item(action, format!("row {action}"), Icon::Check)
}

#[test]
fn esc_with_nothing_open_changes_nothing() {
    let mut f = fixture();
    f.press("escape");
    assert!(f.stack().is_empty());
}

#[test]
fn open_overlay_replaces_the_previous_overlay() {
    let mut f = fixture();
    f.open_menu(vec![item(0)]);
    f.with(|ws, _, cx| ws.open_overlay(Overlay::Settings, cx));
    assert_eq!(f.stack(), ["settings"]);
    f.press("ctrl-b");
    assert_eq!(f.stack(), ["popover"]);
}

#[test]
fn esc_closes_the_picker_while_its_filter_has_focus() {
    let mut f = fixture();
    for key in ["t", "b", "shift-n"] {
        f.press(&format!("{SECONDARY}-{key}"));
        assert_eq!(f.stack(), ["popover"]);
        f.press("escape");
        assert!(f.stack().is_empty(), "{key}");
    }
}

#[test]
fn esc_closes_the_branch_menu_before_the_picker() {
    let mut f = fixture();
    f.press(&format!("{SECONDARY}-b"));
    let branch = f.read(|ws, cx| ws.repo.as_ref().unwrap().read(cx).branches[0].clone());
    f.with(|ws, w, cx| ws.open_branch_menu(branch, point(px(400.0), px(200.0)), w, cx));
    assert_eq!(f.stack(), ["popover", "menu"]);
    f.press("escape");
    assert_eq!(f.stack(), ["popover"]);
    f.press("escape");
    assert!(f.stack().is_empty());
}

#[test]
fn esc_closes_settings() {
    let mut f = fixture();
    f.press(&format!("{SECONDARY}-,"));
    assert_eq!(f.stack(), ["settings"]);
    f.press("escape");
    assert!(f.stack().is_empty());
}

#[test]
fn arrows_move_the_menu_highlight_and_enter_runs_that_row() {
    let mut f = fixture();
    let ran = f.open_menu(vec![item(1), MenuRow::Separator, MenuRow::Item(crate::components::RowItem::new(2, "off", Icon::Check).disabled(true)), item(3)]);
    let selected = |f: &mut Fixture| f.read(|ws, _| matches!(ws.overlays.last(), Some(Overlay::Menu(m)) if m.selected == Some(3)));
    f.press("enter");
    assert!(ran.borrow().is_empty(), "nothing highlighted yet");
    assert_eq!(f.stack(), ["menu"]);
    f.press("up");
    assert!(selected(&mut f), "up from nothing starts at the last row");
    f.press("enter");
    assert_eq!(*ran.borrow(), [3]);
    assert!(f.stack().is_empty(), "the chosen menu closes");
}

#[test]
fn esc_closes_a_menu_without_running_anything() {
    let mut f = fixture();
    let ran = f.open_menu(vec![item(1)]);
    f.press("down");
    f.press("escape");
    assert!(f.stack().is_empty());
    assert!(ran.borrow().is_empty());
}

#[test]
fn backdrop_click_closes_the_picker_and_swallows_the_trigger_click_that_follows() {
    let mut f = fixture();
    f.press(&format!("{SECONDARY}-b"));
    // The picker trigger sits under the scrim: this press is what closes it.
    f.click(320.0, 20.0);
    assert!(f.stack().is_empty());
    let click = |x: f32, y: f32| {
        let (position, modifiers) = (point(px(x), px(y)), Default::default());
        let down = MouseDownEvent { button: MouseButton::Left, position, modifiers, click_count: 1, first_mouse: false };
        let up = MouseUpEvent { button: MouseButton::Left, position, modifiers, click_count: 1 };
        ClickEvent::Mouse(MouseClickEvent { down, up })
    };
    f.with(|ws, w, cx| ws.click_popover_trigger(Popover::Branches, &click(320.0, 20.0), w, cx));
    assert!(f.stack().is_empty(), "same press as the dismissal: stays closed");
    f.with(|ws, w, cx| ws.click_popover_trigger(Popover::Branches, &click(320.0, 20.0), w, cx));
    assert_eq!(f.stack(), ["popover"], "a later click toggles again");
}

#[test]
fn enter_and_esc_work_in_a_discard_dialog() {
    let mut f = fixture();
    let dialog = || Overlay::Discard(crate::features::menus::ConfirmDialog::DiscardAll(Vec::new()));
    f.with(|ws, w, cx| {
        ws.open_overlay(dialog(), cx);
        ws.focus(w, cx)
    });
    f.press("escape");
    assert!(f.stack().is_empty());
    f.with(|ws, _, cx| ws.open_overlay(dialog(), cx));
    f.press("enter");
    assert!(f.stack().is_empty());
}

#[test]
fn stash_dialog_confirms_and_cancels_with_its_input_focused() {
    let mut f = fixture();
    let open = |f: &mut Fixture| {
        f.with(|ws, w, cx| {
            ws.stash_ui.input.update(cx, |s, cx| s.emplace("wip", cx));
            ws.open_overlay(Overlay::Stash(StashDialog::Create { selected_only: false }), cx);
            window_focus(&ws.stash_ui.input, w, cx)
        })
    };
    open(&mut f);
    f.press("escape");
    assert!(f.stack().is_empty());
    open(&mut f);
    f.press("enter");
    assert!(f.stack().is_empty());
}

#[test]
fn branch_dialog_confirms_only_a_valid_name_and_cancels_with_esc() {
    let mut f = fixture();
    let branch = f.read(|ws, cx| ws.repo.as_ref().unwrap().read(cx).branches.iter().find(|b| b.name == "feature").unwrap().clone());
    let open = |f: &mut Fixture, name: &str| {
        f.with(|ws, w, cx| {
            ws.branch_ui.input.update(cx, |s, cx| s.emplace(name, cx));
            ws.open_overlay(Overlay::Branch(BranchDialog::Rename { old: branch.name.clone() }), cx);
            window_focus(&ws.branch_ui.input, w, cx)
        })
    };
    open(&mut f, "feature");
    f.press("enter");
    assert_eq!(f.stack(), ["branch"], "unchanged name is not a rename");
    f.press("escape");
    assert!(f.stack().is_empty());
    open(&mut f, "renamed");
    f.press("enter");
    assert!(f.stack().is_empty());
}

#[test]
fn history_dialog_needs_a_name_to_confirm_and_cancels_with_esc() {
    let mut f = fixture();
    let commit = f.head_commit();
    let open = |f: &mut Fixture, name: &str| {
        let commit = commit.clone();
        f.with(|ws, w, cx| {
            ws.history_ui.name.update(cx, |s, cx| s.emplace(name, cx));
            ws.open_overlay(Overlay::History(HistoryDialog::CreateTag(commit)), cx);
            window_focus(&ws.history_ui.name, w, cx)
        })
    };
    open(&mut f, "");
    f.press("enter");
    assert_eq!(f.stack(), ["history"], "empty name");
    f.press("escape");
    assert!(f.stack().is_empty());
    open(&mut f, "v1");
    f.press("enter");
    assert!(f.stack().is_empty());
}

#[test]
fn merge_dialog_confirms_and_cancels_with_its_filter_focused() {
    let mut f = fixture();
    let open = |f: &mut Fixture| {
        f.with(|ws, w, cx| ws.open_merge_dialog(MergeMode::Merge, Some("feature".into()), w, cx));
        f.wait(|f| f.read(|ws, _| matches!(ws.overlays.last(), Some(Overlay::Merge(d)) if d.preview.as_ref().is_some_and(|p| p.ahead > 0))));
    };
    open(&mut f);
    f.press("escape");
    assert!(f.stack().is_empty());
    open(&mut f);
    f.press("enter");
    assert!(f.stack().is_empty());
}

fn window_focus(input: &gpui::Entity<gpui_elements::editable_text::EditableTextState>, w: &mut Window, cx: &mut gpui::App) {
    w.focus(&input.focus_handle(cx), cx)
}
