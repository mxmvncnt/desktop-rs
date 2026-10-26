//! File-system watcher for the open repository (see docs/ARCHITECTURE.md
//! §File watcher).
//!
//! `notify-debouncer-full` owns the OS thread and the debounce; it sleeps
//! when idle (zero CPU). A tick is sent to the UI side once a burst of events
//! has been quiet for [`DEBOUNCE`]. `bool` payload: `true` when git metadata
//! (HEAD, index, refs, ...) changed too, so branches/history need reloading;
//! `false` when only working-tree files changed.
//!
//! Dropping [`RepoWatcher`] stops the thread and drops the OS watches.
//!
//! Layout of the watches (instead of one recursive watch on the root, which
//! would also cover `.git/objects` with its 256 fan-out dirs and build dirs):
//! - the root, non-recursive (to notice new top-level entries),
//! - every top-level directory except `.git` and the ones `.gitignore` ignores,
//!   recursively,
//! - the git dir non-recursively plus `refs/` recursively.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gd_git::Repository;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify::{EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};

/// Quiet period after the last event before a refresh is requested.
pub const DEBOUNCE: Duration = Duration::from_millis(300);

type Deb = Debouncer<RecommendedWatcher, RecommendedCache>;

/// Files directly inside the git dir whose changes matter.
const GIT_FILES: [&str; 11] = [
    "HEAD",
    "index",
    "FETCH_HEAD",
    "MERGE_HEAD",
    "CHERRY_PICK_HEAD",
    "REVERT_HEAD",
    "REBASE_HEAD",
    "ORIG_HEAD",
    "packed-refs",
    "MERGE_MSG",
    "config",
];

/// Shared with the event handler, which adds watches for new top-level dirs.
pub struct RepoWatcher {
    debouncer: Arc<Mutex<Option<Deb>>>,
}

impl Drop for RepoWatcher {
    fn drop(&mut self) {
        // Dropped outside the lock: it joins the debouncer thread, whose handler
        // only ever `try_lock`s.
        let debouncer = self.debouncer.lock().ok().and_then(|mut d| d.take());
        drop(debouncer);
    }
}

#[derive(Clone)]
struct Paths {
    root: PathBuf,
    git_dir: PathBuf,
    common_dir: PathBuf,
    ignore: Gitignore,
}

impl RepoWatcher {
    /// Start watching. Ticks arrive on the returned receiver; it closes when
    /// the watcher stops.
    pub fn start(repo: &Repository) -> (Self, smol::channel::Receiver<bool>) {
        let (out, rx) = smol::channel::unbounded::<bool>();
        let paths = Paths {
            root: repo.root().to_path_buf(),
            git_dir: repo.git_dir(),
            common_dir: repo.common_git_dir(),
            ignore: gitignore(repo.root(), &repo.common_git_dir()),
        };
        let slot = Arc::new(Mutex::new(None));
        let (handler_paths, handler_slot) = (paths.clone(), slot.clone());
        let handler = move |result: DebounceEventResult| {
            let events = match result {
                Ok(events) => events,
                Err(errors) => return log::debug!("fs watcher errors: {errors:?}"),
            };
            let mut hit: Option<bool> = None;
            for ev in events {
                log::trace!("fs event: {:?}", ev.paths);
                // Reads (`git status` opening files) must never count as changes,
                // otherwise our own refresh would retrigger itself.
                if !matches!(ev.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any) {
                    continue;
                }
                for p in &ev.paths {
                    let Some(full) = classify(&handler_paths, p) else { continue };
                    hit = Some(hit.unwrap_or(false) || full);
                    // A new top-level directory needs its own recursive watch.
                    if matches!(ev.kind, EventKind::Create(_))
                        && p.parent() == Some(handler_paths.root.as_path())
                        && let Ok(mut guard) = handler_slot.try_lock()
                        && let Some(d) = guard.as_mut()
                    {
                        watch_top_level(d, &handler_paths, p);
                    }
                }
            }
            if let Some(full) = hit {
                log::debug!("fs watcher tick (full={full})");
                let _ = out.send_blocking(full);
            }
        };
        match new_debouncer(DEBOUNCE, None, handler) {
            Ok(mut debouncer) => {
                add_initial_watches(&mut debouncer, &paths);
                *slot.lock().unwrap() = Some(debouncer);
            }
            Err(e) => log::warn!("fs watcher unavailable: {e}"),
        }
        (Self { debouncer: slot }, rx)
    }
}

/// The repository's own ignore rules (`.gitignore` at the root, `info/exclude`).
fn gitignore(root: &Path, git_dir: &Path) -> Gitignore {
    let mut builder = GitignoreBuilder::new(root);
    builder.add(root.join(".gitignore"));
    builder.add(git_dir.join("info/exclude"));
    builder.build().unwrap_or_else(|_| Gitignore::empty())
}

fn add_initial_watches(d: &mut Deb, p: &Paths) {
    let _ = d.watch(&p.root, RecursiveMode::NonRecursive);
    if let Ok(rd) = std::fs::read_dir(&p.root) {
        for entry in rd.flatten() {
            watch_top_level(d, p, &entry.path());
        }
    }
    for dir in [&p.git_dir, &p.common_dir] {
        let _ = d.watch(dir, RecursiveMode::NonRecursive);
        let refs = dir.join("refs");
        if refs.is_dir() {
            let _ = d.watch(&refs, RecursiveMode::Recursive);
        }
        if p.git_dir == p.common_dir {
            break;
        }
    }
}

fn watch_top_level(d: &mut Deb, p: &Paths, path: &Path) {
    // `is_dir` follows symlinks; don't recurse through links (cycles).
    let is_real_dir = std::fs::symlink_metadata(path).map(|m| m.is_dir()).unwrap_or(false);
    let ignored = path.file_name().is_some_and(|n| n == ".git" || p.ignore.matched(n, true).is_ignore());
    if is_real_dir && !ignored {
        let _ = d.watch(path, RecursiveMode::Recursive);
    }
}

/// `Some(full)` if an event on `path` should trigger a refresh.
fn classify(p: &Paths, path: &Path) -> Option<bool> {
    for dir in [&p.git_dir, &p.common_dir] {
        if let Ok(rel) = path.strip_prefix(dir) {
            return classify_git(rel).then_some(true);
        }
    }
    let rel = path.strip_prefix(&p.root).ok()?;
    if rel.starts_with(".git") || p.ignore.matched_path_or_any_parents(rel, false).is_ignore() {
        return None;
    }
    Some(false)
}

/// `rel` is relative to the git dir.
fn classify_git(rel: &Path) -> bool {
    let mut comps = rel.components().map(|c| c.as_os_str().to_str().unwrap_or(""));
    match comps.next() {
        Some("refs") => !rel.to_str().is_some_and(|s| s.ends_with(".lock")),
        Some(name) if comps.next().is_none() => {
            GIT_FILES.contains(&name) || name.starts_with("rebase-")
        }
        // objects/, logs/, hooks/, lfs/, modules/ ... and any nested path.
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths() -> Paths {
        Paths {
            root: "/r".into(),
            git_dir: "/r/.git".into(),
            common_dir: "/r/.git".into(),
            ignore: {
                let mut b = GitignoreBuilder::new("/r");
                b.add_line(None, "target/").unwrap();
                b.add_line(None, "node_modules/").unwrap();
                b.build().unwrap()
            },
        }
    }

    #[test]
    fn classifies() {
        let p = paths();
        assert_eq!(classify(&p, Path::new("/r/src/a.rs")), Some(false));
        assert_eq!(classify(&p, Path::new("/r/target/debug/x")), None);
        assert_eq!(classify(&p, Path::new("/r/web/node_modules/x/y")), None);
        assert_eq!(classify(&p, Path::new("/r/.git/index")), Some(true));
        assert_eq!(classify(&p, Path::new("/r/.git/index.lock")), None);
        assert_eq!(classify(&p, Path::new("/r/.git/HEAD")), Some(true));
        assert_eq!(classify(&p, Path::new("/r/.git/refs/heads/main")), Some(true));
        assert_eq!(classify(&p, Path::new("/r/.git/refs/heads/main.lock")), None);
        assert_eq!(classify(&p, Path::new("/r/.git/objects/ab/cdef")), None);
        assert_eq!(classify(&p, Path::new("/r/.git/logs/HEAD")), None);
        assert_eq!(classify(&p, Path::new("/r/.git/FETCH_HEAD")), Some(true));
        assert_eq!(classify(&p, Path::new("/r/.git/rebase-merge")), Some(true));
    }
}
