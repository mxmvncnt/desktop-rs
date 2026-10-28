//! Headless check of the file watcher through a real `Workspace` (needs `--features snapshot` for
//! GPUI's test platform). Asserts: a file edit shows up in the status without any manual refresh,
//! a burst is coalesced (a few refreshes), and an idle repo triggers no further refresh (no
//! feedback loop) and ~zero CPU. Real sleeps are required: the watcher thread debounces in
//! wall-clock time, and the test dispatcher only runs tasks when `run_until_parked` is called.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{AppContext as _, HeadlessAppContext, px, size};

use crate::workspace::Workspace;

fn git(repo: &Path, args: &[&str]) {
    let out = Command::new("git").args(args).current_dir(repo).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

/// utime + stime in clock ticks (100/s) from /proc/self/stat (fields 14, 15).
fn process_cpu_ticks() -> u64 {
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
    let fields: Vec<u64> = stat.rsplit_once(')').map_or("", |x| x.1).split_whitespace().skip(11).take(2).filter_map(|v| v.parse().ok()).collect();
    fields.iter().sum()
}

#[test]
fn watcher_updates_status_without_feedback_loop() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    git(repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("a.txt"), "a\n").unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["-c", "user.name=T", "-c", "user.email=t@example.com", "commit", "-qm", "init"]);

    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(gd_ui::Assets), || None);
    cx.update(gd_ui::init);
    let window = cx
        .open_window(size(px(1280.0), px(820.0)), |window, cx| cx.new(|cx| Workspace::new(Some(repo.to_path_buf()), window, cx)))
        .unwrap();
    let m = cx.update_window(window.into(), |_, w, cx| w.root::<Workspace>().flatten().unwrap().read(cx).repo.clone().unwrap()).unwrap();
    let settle = |cx: &mut HeadlessAppContext, ms: u64| {
        let end = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < end {
            std::thread::sleep(Duration::from_millis(50));
            cx.run_until_parked();
        }
    };
    let count = |cx: &mut HeadlessAppContext| cx.read_entity(&m, |r, _| r.sync.refresh_count);
    let nfiles = |cx: &mut HeadlessAppContext| cx.read_entity(&m, |r, _| r.changes().len());
    settle(&mut cx, 1000);

    // 1. Idle: no refreshes, no CPU.
    let (c0, t0) = (count(&mut cx), process_cpu_ticks());
    settle(&mut cx, 3000);
    let (c1, t1) = (count(&mut cx), process_cpu_ticks());
    println!("idle 3s: refreshes {c0} -> {c1}, cpu ticks {}", t1 - t0);
    assert_eq!(c0, c1, "refresh feedback loop while idle");

    // 2. A burst of edits -> a few debounced refreshes, status updated.
    let before = nfiles(&mut cx);
    for i in 0..20 {
        std::fs::write(repo.join(format!("watch-{i}.txt")), format!("{i}\n")).unwrap();
        std::thread::sleep(Duration::from_millis(10));
    }
    let t = Instant::now();
    while nfiles(&mut cx) == before && t.elapsed() < Duration::from_secs(5) {
        settle(&mut cx, 50);
    }
    settle(&mut cx, 1000);
    let (c2, after) = (count(&mut cx), nfiles(&mut cx));
    assert_eq!(after, before + 20, "watcher did not update status");
    assert!(c2 <= c1 + 3, "burst was not coalesced");

    // 3. Git metadata change (new branch via CLI) -> full refresh sees it.
    git(repo, &["branch", "watch-branch"]);
    settle(&mut cx, 1500);
    assert!(cx.read_entity(&m, |r, _| r.branches.iter().any(|b| b.name == "watch-branch")), "ref change not picked up");

    // 4. Deleting files again; then idle once more.
    for i in 0..20 {
        std::fs::remove_file(repo.join(format!("watch-{i}.txt"))).unwrap();
    }
    git(repo, &["branch", "-D", "watch-branch"]);
    settle(&mut cx, 1500);
    assert_eq!(nfiles(&mut cx), before);
    let c4 = count(&mut cx);
    settle(&mut cx, 2500);
    assert_eq!(c4, count(&mut cx), "refresh feedback loop after changes");
}
