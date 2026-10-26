//! `gd-git` — the git layer for desktop-rs.
//!
//! Design (see `docs/ARCHITECTURE.md` §Git layer):
//! - Shells out to the user's `git` binary, the same strategy as the Electron
//!   app's `dugite`. Hooks, credential helpers, config and LFS behave exactly
//!   like the CLI, and we avoid libgit2's partial feature coverage.
//! - Every function is **blocking**. The UI runs them on GPUI's background
//!   executor (`cx.background_spawn`) and never on the main thread.
//! - Machine-readable formats only (`status --porcelain=v2 -z`, NUL-separated
//!   `log` formats) so parsing is cheap and locale-independent.

mod credentials;
mod diff;
pub mod engine;
mod exec;
mod expand;
mod history_ops;
mod identity;
mod log;
mod merge;
mod name_status;
mod partial;
mod pr_preview;
mod progress;
mod refs;
mod remote;
mod stash;
mod status;

pub use credentials::{HostCredential, set_credentials};
pub use identity::{EffectiveIdentity, Identity, IdentitySource, global_identity, set_global_identity};

pub use diff::{Side, Diff, DiffHunk, DiffLine, DiffLineKind, DiffOptions};
pub use exec::GitError;
pub use expand::{EXPANSION_STEP, Expansion, expand, expandable, gap_above, gap_below};
pub use history_ops::ResetMode;
pub use log::{CoAuthor, Commit};
pub use merge::{MergePreview, default_squash_summary};
pub use partial::{PartialFile, selectable, staged_text};
pub use pr_preview::{Comparison, compare_url};
pub use remote::{Host, github_web_url, host_of};
pub use progress::{InProgress, OpOutcome};
pub use refs::{Branch, BranchKind};
pub use stash::Stash;
pub use status::{AheadBehind, FileChange, FileStatus, WorkingStatus};

use std::path::{Path, PathBuf};

use exec::Git;

pub type Result<T> = std::result::Result<T, GitError>;

/// A handle to a repository on disk. Cheap to clone; holds no open resources.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Repository {
    root: PathBuf,
}

impl Repository {
    /// Open the repository containing `path` (any subdirectory works).
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let out = Git::new(path.as_ref())
            .args(["rev-parse", "--show-toplevel"])
            .run()?;
        let root = PathBuf::from(out.trim_end_matches(['\n', '\r']));
        Ok(Self { root })
    }

    /// Initialize a new repository at `path` (creating the directory).
    pub fn init(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        std::fs::create_dir_all(path).map_err(|e| GitError::Io(e.to_string()))?;
        Git::new(path).args(["init", "-q"]).run()?;
        Self::open(path)
    }

    /// Clone `url` into `dest`. Blocks for the whole transfer.
    pub fn clone(url: &str, dest: impl AsRef<Path>) -> Result<Self> {
        let dest = dest.as_ref();
        let parent = dest.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(|e| GitError::Io(e.to_string()))?;
        Git::new(parent)
            .args(["clone", "--progress", "--", url])
            .arg(dest.as_os_str())
            .network()
            .run()?;
        Self::open(dest)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Directory name, used as the display name in the repository list.
    pub fn name(&self) -> String {
        self.root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.root.display().to_string())
    }

    /// Fresh libgit2 handle for reads. Open one per call: it caches the index.
    pub(crate) fn repo(&self) -> Result<git2::Repository> {
        Ok(git2::Repository::open(&self.root)?)
    }

    pub(crate) fn git(&self) -> Git {
        Git::new(&self.root)
    }

    /// Whether `HEAD` resolves to a commit (false in a freshly-initialized repo).
    pub fn has_head(&self) -> bool {
        self.repo().is_ok_and(|r| r.head().is_ok())
    }

    // ---- write operations -------------------------------------------------

    /// Commit exactly `paths` with the given message, mirroring Desktop's
    /// "checkbox" model: the index is reset to HEAD and only the selected
    /// paths are staged, so whatever the user ticked is what gets committed.
    pub fn commit(&self, summary: &str, description: &str, paths: &[String]) -> Result<String> {
        self.commit_partial(summary, description, paths, &[])
    }

    /// [`Self::commit`] plus files of which only some lines are committed
    /// (`partial`, not listed in `paths`).
    pub fn commit_partial(
        &self,
        summary: &str,
        description: &str,
        paths: &[String],
        partial: &[PartialFile],
    ) -> Result<String> {
        if summary.trim().is_empty() {
            return Err(GitError::Invalid("commit summary is empty".into()));
        }
        if paths.is_empty() && partial.is_empty() {
            return Err(GitError::Invalid("no files selected".into()));
        }
        if self.has_head() {
            self.git().args(["reset", "-q", "--", "."]).run()?;
        } else {
            // No HEAD yet: unstage everything by removing it from the index only.
            let _ = self
                .git()
                .args(["rm", "-q", "-r", "--cached", "--ignore-unmatch", "--", "."])
                .run();
        }
        self.stage(paths, partial)?;
        self.git()
            .args(["commit", "-q", "-F", "-"])
            .stdin(message(summary, description))
            .exec()?;
        self.head_sha()
    }

    /// Stage `paths` (adds, edits and deletions) and the selected lines of
    /// `partial` files. Paths go through stdin, so the list length is unbounded.
    pub(crate) fn stage(&self, paths: &[String], partial: &[PartialFile]) -> Result<()> {
        if !paths.is_empty() {
            self.git()
                .args(["add", "-A", "--pathspec-from-file=-", "--pathspec-file-nul"])
                .stdin(paths.join("\0"))
                .exec()?;
        }
        self.stage_partial(partial)
    }

    pub(crate) fn head_sha(&self) -> Result<String> {
        Ok(self.git().args(["rev-parse", "HEAD"]).run()?.trim().to_string())
    }

    /// Discard working-tree changes for `paths`. Untracked files are deleted.
    pub fn discard(&self, changes: &[FileChange]) -> Result<()> {
        let (untracked, tracked): (Vec<_>, Vec<_>) = changes
            .iter()
            .partition(|c| c.status == FileStatus::Untracked);
        if !tracked.is_empty() {
            let paths: Vec<&str> = tracked.iter().map(|c| c.path.as_str()).collect();
            if self.has_head() {
                self.git()
                    .args(["checkout", "-q", "HEAD", "--"])
                    .args(paths.iter().copied())
                    .run()?;
            } else {
                self.git()
                    .args(["rm", "-q", "--cached", "--"])
                    .args(paths.iter().copied())
                    .run()?;
            }
        }
        for c in untracked {
            let p = self.root.join(&c.path);
            let res = if p.is_dir() {
                std::fs::remove_dir_all(&p)
            } else {
                std::fs::remove_file(&p)
            };
            res.map_err(|e| GitError::Io(format!("{}: {e}", c.path)))?;
        }
        Ok(())
    }

    pub fn fetch(&self) -> Result<()> {
        // Fetch the default remote strictly, then the others best-effort, so
        // a broken secondary remote (e.g. a mistyped `upstream` URL) doesn't
        // fail the whole fetch like `git fetch --all` would.
        let default = self.default_remote()?;
        let Some(default) = default else { return Ok(()) };
        self.git()
            .args(["fetch", "--prune", &default])
            .network()
            .run()?;
        for (name, _) in self.remotes()?.into_iter().filter(|(n, _)| *n != default) {
            if let Err(e) = self.git().args(["fetch", "--prune", &name]).network().run() {
                ::log::warn!("fetch of remote '{name}' failed: {e}");
            }
        }
        Ok(())
    }

    pub fn pull(&self) -> Result<()> {
        self.git().args(["pull", "--ff", "--no-edit"]).network().exec()
    }

    /// Fetch, then push only if the remote has no commits we lack. Returns
    /// `false` (without pushing) when the branch is behind its upstream, so
    /// the caller can offer a pull first.
    pub fn push_if_up_to_date(&self, branch: &str) -> Result<bool> {
        self.fetch()?;
        if self.status()?.ahead_behind.behind > 0 {
            return Ok(false);
        }
        self.push(branch, true)?;
        Ok(true)
    }

    /// Push the current branch, setting upstream on first push like Desktop's
    /// "Publish branch".
    pub fn push(&self, branch: &str, has_upstream: bool) -> Result<()> {
        if has_upstream {
            self.git().args(["push"]).network().run()?;
        } else {
            let remote = self.default_remote()?.unwrap_or_else(|| "origin".into());
            self.git()
                .args(["push", "--set-upstream", &remote, branch])
                .network()
                .run()?;
        }
        Ok(())
    }

    /// Append `patterns` to the repository's root `.gitignore` (created if
    /// missing). Patterns already present are skipped. Use [`escape_ignore`]
    /// for literal paths.
    pub fn ignore(&self, patterns: &[String]) -> Result<()> {
        use std::io::Write;
        let file = self.root.join(".gitignore");
        let existing = std::fs::read_to_string(&file).unwrap_or_default();
        let mut out = String::new();
        if !existing.is_empty() && !existing.ends_with('\n') {
            out.push('\n');
        }
        for p in patterns {
            if !existing.lines().any(|l| l == p) && !out.lines().any(|l| l == p) {
                out.push_str(p);
                out.push('\n');
            }
        }
        if out.trim().is_empty() {
            return Ok(());
        }
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file)
            .and_then(|mut f| f.write_all(out.as_bytes()))
            .map_err(|e| GitError::Io(format!(".gitignore: {e}")))
    }

    /// Undo the last commit, keeping its changes in the working tree.
    pub fn undo_last_commit(&self) -> Result<()> {
        self.git().args(["reset", "-q", "--soft", "HEAD~1"]).exec()
    }

    pub fn remotes(&self) -> Result<Vec<(String, String)>> {
        let out = self.git().args(["remote", "-v"]).run()?;
        let mut v: Vec<(String, String)> = out
            .lines()
            .filter(|l| l.ends_with("(fetch)"))
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                Some((it.next()?.to_string(), it.next()?.to_string()))
            })
            .collect();
        v.dedup();
        Ok(v)
    }

    fn default_remote(&self) -> Result<Option<String>> {
        let remotes = self.remotes()?;
        Ok(remotes
            .iter()
            .find(|(n, _)| n == "origin")
            .or(remotes.first())
            .map(|(n, _)| n.clone()))
    }
}

// ---- filesystem-level helpers (no process spawn) ----------------------------

impl Repository {
    /// The repository's git directory (`<root>/.git`, or the target of a
    /// `.git` *file* as used by linked worktrees and submodules). Reads the
    /// filesystem only, so it is cheap and never spawns git.
    pub fn git_dir(&self) -> PathBuf {
        resolve_git_dir(&self.root)
    }

    /// The directory shared between linked worktrees (holds `refs`,
    /// `FETCH_HEAD`, `config`). Equal to [`git_dir`](Self::git_dir) for a
    /// normal repository.
    pub fn common_git_dir(&self) -> PathBuf {
        let dir = self.git_dir();
        match std::fs::read_to_string(dir.join("commondir")) {
            Ok(s) => {
                let p = PathBuf::from(s.trim());
                if p.is_absolute() { p } else { dir.join(p) }
            }
            Err(_) => dir,
        }
    }

    /// When the repository last fetched, from the mtime of `FETCH_HEAD`
    /// (what Desktop does too). `None` if it never fetched. Rewritten by every
    /// `git fetch`, even when nothing new arrived.
    pub fn last_fetch_time(&self) -> Option<std::time::SystemTime> {
        std::fs::metadata(self.common_git_dir().join("FETCH_HEAD"))
            .and_then(|m| m.modified())
            .ok()
    }
}

/// `<root>/.git`, or the target of a `.git` *file* (`gitdir: <path>`, used by
/// linked worktrees and submodules).
pub(crate) fn resolve_git_dir(root: &Path) -> PathBuf {
    let dot_git = root.join(".git");
    if dot_git.is_file()
        && let Ok(text) = std::fs::read_to_string(&dot_git)
        && let Some(target) = text.trim().strip_prefix("gitdir:")
    {
        return root.join(target.trim());
    }
    dot_git
}

/// Commit message: the trimmed summary, then the description after a blank line.
pub(crate) fn message(summary: &str, description: &str) -> String {
    match description.trim() {
        "" => summary.trim().to_string(),
        d => format!("{}\n\n{d}", summary.trim()),
    }
}

/// Escape gitignore metacharacters so `path` matches literally, anchored at
/// the repository root (`/dir/file name.txt`).
pub fn escape_ignore(path: &str) -> String {
    let mut out = String::from("/");
    for (i, c) in path.chars().enumerate() {
        if matches!(c, '*' | '?' | '[' | ']' | '\\') || (i == 0 && matches!(c, '#' | '!')) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
