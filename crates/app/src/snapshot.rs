//! Headless screenshot mode (feature `snapshot`).
//!
//! Renders the real `Workspace` offscreen through GPUI-CE's headless WGPU
//! renderer and writes a PNG. Used by humans and AI sessions to check visuals
//! without a display. Git work runs on the deterministic test dispatcher, so
//! `run_until_parked` waits for status/diff/history loads to finish.
//!
//! ```sh
//! GD_CONFIG_DIR=/tmp/desktop-rs/config cargo run --features snapshot -- \
//!     --snapshot /tmp/desktop-rs/shot.png /tmp/desktop-rs/testrepo history
//! ```
//!
//! A view is a name plus the `Step`s that put the `Workspace` in that state. Each feature lists
//! its own in `features/<feature>/snapshot.rs` (`views()`) and `FEATURES` below collects them.
//! A name ending in `*` matches by prefix; the full name is passed to the steps (`changes@3`).
//! Steps run in order with the pending loads finished in between. An unknown name renders the
//! plain workspace (`changes`, `welcome`, `stash-list`). `icons` is an icon-rendering check that
//! ignores the repo.
//!
//! Env (logical window px): `GD_SNAPSHOT_MOUSE=x,y` parks the pointer there to capture hover
//! states; `GD_SNAPSHOT_DRAG=x1,y1,x2,y2` drags with the left button, e.g. to select diff text.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use gpui::{AppContext as _, Context, HeadlessAppContext, Window, px, size};

use crate::actions::SelectNext;
use crate::features::{accounts, branches, changes, diff, history, menus, merge, pickers, pr_preview, settings, stashes};
use crate::workspace::{NavList, Workspace};

pub type Step = fn(&mut Workspace, &str, &mut Window, &mut Context<Workspace>);

pub struct View {
    pub name: &'static str,
    pub steps: &'static [Step],
}

const FEATURES: [fn() -> &'static [View]; 11] = [
    accounts::snapshot::views,
    branches::snapshot::views,
    changes::snapshot::views,
    diff::snapshot::views,
    history::snapshot::views,
    menus::snapshot::views,
    merge::snapshot::views,
    pickers::snapshot::views,
    pr_preview::snapshot::views,
    settings::snapshot::views,
    stashes::snapshot::views,
];

fn find(view: &str) -> Option<&'static View> {
    FEATURES.iter().flat_map(|views| views()).find(|v| match v.name.strip_suffix('*') {
        Some(prefix) => view.starts_with(prefix),
        None => v.name == view,
    })
}

/// Focus a list and press Down in it, like the keyboard would.
pub fn press_next(list: Option<NavList>, ws: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.nav_list = list;
    ws.focus(window, cx);
    // The action handler updates the workspace, so it must run once this update is over.
    window.defer(cx, |window, cx| window.dispatch_action(Box::new(SelectNext), cx));
}

pub fn run(out: PathBuf, repo: Option<PathBuf>, view: &str) {
    let platform = gpui_platform::current_platform(true);
    let text_system = platform.text_system();
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(gd_ui::Assets), || {
        Some(Box::new(
            gpui_wgpu::WgpuHeadlessRenderer::new().expect("headless WGPU renderer"),
        ) as Box<dyn gpui::PlatformHeadlessRenderer>)
    });
    cx.update(|cx| {
        gd_ui::init(cx);
        crate::actions::bind(cx);
    });
    if view == "icons" {
        struct Icons;
        impl gpui::Render for Icons {
            fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
                use gpui::{ParentElement, Styled};
                gpui::div().size_full().bg(gpui::black()).flex().gap(px(10.0)).p(px(20.0))
                    .child(gd_ui::icon(gd_ui::Icon::Check).size(px(64.0)).text_color(gpui::white()))
                    .child(gd_ui::icon(gd_ui::Icon::GitBranch).size(px(64.0)).text_color(gpui::white()))
                    .child(gpui::svg().path("icons/check.svg").w(px(64.0)).h(px(64.0)).text_color(gpui::red()))
                    .child("text")
            }
        }
        let window = cx.open_window(size(px(400.0), px(120.0)), |_, cx| cx.new(|_| Icons)).unwrap();
        cx.run_until_parked();
        let counts = cx.update_window(window.into(), |_, w, _| w.rendered_primitive_counts()).ok();
        eprintln!("icon counts {counts:?}");
        cx.capture_screenshot(window.into()).unwrap().save(&out).unwrap();
        return;
    }
    let repo = if view == "welcome" || view.ends_with("-norepo") { None } else { repo };
    let window = cx
        .open_window(size(px(1280.0), px(820.0)), |window, cx| {
            cx.new(|cx| Workspace::new(repo, window, cx))
        })
        .expect("open headless window");
    cx.run_until_parked();

    for (i, step) in find(view).map_or(&[][..], |v| v.steps).iter().enumerate() {
        if i > 0 {
            for _ in 0..3 {
                std::thread::sleep(Duration::from_millis(250));
                cx.run_until_parked();
            }
        }
        cx.update_window(window.into(), |_, window, cx| {
            let ws = window.root::<Workspace>().flatten().expect("workspace root");
            ws.update(cx, |ws, cx| step(ws, view, window, cx));
        })
        .ok();
    }
    cx.run_until_parked();
    // `GD_SNAPSHOT_MOUSE=x,y` (window px): park the pointer there to capture hover states.
    if let Some((x, y)) = std::env::var("GD_SNAPSHOT_MOUSE")
        .ok()
        .and_then(|v| v.split_once(',').and_then(|(x, y)| Some((x.parse::<f32>().ok()?, y.parse::<f32>().ok()?))))
    {
        cx.run_until_parked();
        for _ in 0..2 {
            cx.update_window(window.into(), |_, window, cx| {
                window.dispatch_event(
                    gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
                        position: gpui::point(px(x), px(y)),
                        pressed_button: None,
                        modifiers: Default::default(),
                    }),
                    cx,
                );
            })
            .ok();
            cx.run_until_parked();
        }
    }
    // `GD_SNAPSHOT_DRAG=x1,y1,x2,y2` (window px): press, drag and release the
    // left button, e.g. to select diff text through the real hit-testing.
    if let Some(&[x1, y1, x2, y2]) = std::env::var("GD_SNAPSHOT_DRAG")
        .ok()
        .and_then(|v| v.split(',').map(|n| n.parse::<f32>().ok()).collect::<Option<Vec<_>>>())
        .as_deref()
    {
        use gpui::{MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PlatformInput, point};
        let (from, to) = (point(px(x1), px(y1)), point(px(x2), px(y2)));
        let left = Some(MouseButton::Left);
        let events = [
            PlatformInput::MouseMove(MouseMoveEvent { position: from, pressed_button: None, modifiers: Default::default() }),
            PlatformInput::MouseDown(MouseDownEvent { button: MouseButton::Left, position: from, modifiers: Default::default(), click_count: 1, first_mouse: false }),
            PlatformInput::MouseMove(MouseMoveEvent { position: point(px((x1 + x2) / 2.0), px((y1 + y2) / 2.0)), pressed_button: left, modifiers: Default::default() }),
            PlatformInput::MouseMove(MouseMoveEvent { position: to, pressed_button: left, modifiers: Default::default() }),
            PlatformInput::MouseUp(MouseUpEvent { button: MouseButton::Left, position: to, modifiers: Default::default(), click_count: 1 }),
        ];
        cx.run_until_parked();
        for event in events {
            cx.update_window(window.into(), |_, window, cx| {
                window.dispatch_event(event, cx);
            })
            .ok();
            cx.run_until_parked();
        }
    }
    // Let loads finish and entrance animations settle.
    for _ in 0..4 {
        cx.run_until_parked();
        cx.advance_clock(Duration::from_millis(600));
    }
    cx.run_until_parked();
    // GPUI animations/transitions run on wall-clock time (not the test
    // dispatcher clock): wait for them for real, then redraw.
    for _ in 0..3 {
        std::thread::sleep(Duration::from_millis(300));
        cx.update_window(window.into(), |_, window, _| window.refresh()).ok();
        cx.run_until_parked();
    }

    let image = cx.capture_screenshot(window.into()).expect("capture");
    image.save(&out).expect("save png");
    println!("wrote {}", out.display());
}
