//! Diff model + loading. git only fetches the two file versions; the diff
//! itself is computed in-process by `engine.rs` (the `similar` crate).
//!
//! The UI renders diffs with a virtualized `uniform_list`, so the parsed form
//! is a flat `Vec<DiffLine>` (hunk headers included) rather than nested hunks:
//! row `i` of the list is `lines[i]`, O(1).

use crate::engine::hunk_header;
use crate::{FileChange, FileStatus, Repository, Result};

/// How a diff is produced (app-wide display options).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DiffOptions {
    /// Ignore all whitespace (like `git diff -w`): hide whitespace-only changes.
    pub ignore_whitespace: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Hunk,
    Context,
    Add,
    Delete,
    /// "\ No newline at end of file"
    NoNewline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    /// Line content without the leading `+`/`-`/` ` marker.
    pub text: String,
    /// Byte ranges of `text` that changed within a paired delete/add row
    /// (word-level emphasis, computed by the engine).
    pub emphasis: Vec<(usize, usize)>,
}

impl DiffLine {
    /// A numbered row (0-based line indexes); tabs are expanded once here
    /// instead of at every paint.
    pub(crate) fn new(kind: DiffLineKind, old_ix: Option<usize>, new_ix: Option<usize>, text: &str) -> Self {
        DiffLine {
            kind,
            old_line: old_ix.map(|i| i as u32 + 1),
            new_line: new_ix.map(|i| i as u32 + 1),
            text: if text.contains('\t') { text.replace('\t', "    ") } else { text.to_string() },
            emphasis: Vec::new(),
        }
    }

    pub(crate) fn hunk(h: &DiffHunk) -> Self {
        let text = hunk_header(h.old_start, h.old_len, h.new_start, h.new_len);
        DiffLine { kind: DiffLineKind::Hunk, old_line: None, new_line: None, text, emphasis: Vec::new() }
    }

    pub(crate) fn no_newline() -> Self {
        let text = "\\ No newline at end of file".into();
        DiffLine { kind: DiffLineKind::NoNewline, old_line: None, new_line: None, text, emphasis: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    /// Index into [`Diff::lines`] of the hunk header.
    pub header_index: usize,
    /// 0-based first line and line count on each side (what the header
    /// shows, 0-based). Used to compute the unchanged gaps for expansion.
    pub old_start: usize,
    pub old_len: usize,
    pub new_start: usize,
    pub new_len: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diff {
    pub lines: Vec<DiffLine>,
    pub hunks: Vec<DiffHunk>,
    pub is_binary: bool,
    pub truncated: bool,
    /// Index of the row with the most characters; the UI measures this row so
    /// the horizontal scroll extent covers the widest line, not just row 0.
    pub longest_line: usize,
    pub added: u32,
    pub deleted: u32,
    /// A side exceeded `engine::MAX_BYTES`; nothing was diffed.
    pub too_large: bool,
    /// Full old/new texts the diff came from (for partial staging). `None`
    /// for binary or non-UTF-8 files.
    pub source: Option<std::sync::Arc<crate::engine::DiffSource>>,
}

/// One side of a diff: where to read the file version from.
#[derive(Debug, Clone)]
pub enum Side {
    /// The file doesn't exist on this side (added / deleted).
    Empty,
    /// `git cat-file blob <rev>:<path>`. A path missing at `rev` reads as empty.
    Blob { rev: String, path: String },
    /// The working-tree file (symlinks read as their target, like git).
    Worktree(String),
}

/// Sides of `file` between two revisions: empty before an added file and
/// after a deleted one.
pub(crate) fn blob_sides(old_rev: &str, new_rev: &str, file: &FileChange) -> (Side, Side) {
    let old = if file.status == FileStatus::New {
        Side::Empty
    } else {
        Side::Blob {
            rev: old_rev.to_string(),
            path: file.old_path.clone().unwrap_or_else(|| file.path.clone()),
        }
    };
    let new = if file.status == FileStatus::Deleted {
        Side::Empty
    } else {
        Side::Blob { rev: new_rev.to_string(), path: file.path.clone() }
    };
    (old, new)
}

impl Repository {
    /// Diff of a working-directory change against HEAD (index + worktree),
    /// computed in-process with `similar` (see `engine.rs`).
    pub fn working_diff(&self, change: &FileChange, opts: DiffOptions) -> Result<Diff> {
        let old = match change.status {
            FileStatus::Untracked => Side::Empty,
            _ if !self.has_head() => Side::Empty,
            _ => Side::Blob {
                rev: "HEAD".into(),
                path: change.old_path.clone().unwrap_or_else(|| change.path.clone()),
            },
        };
        let new = match change.status {
            FileStatus::Deleted => Side::Empty,
            _ => Side::Worktree(change.path.clone()),
        };
        self.diff_sides(&old, &new, opts)
    }

    /// Diff of one file within a commit (against its first parent).
    pub fn commit_diff(&self, sha: &str, file: &FileChange, opts: DiffOptions) -> Result<Diff> {
        let has_parent = self
            .git()
            .args(["rev-parse", "--verify", "-q"])
            .arg(format!("{sha}^1"))
            .run()
            .is_ok();
        let (mut old, new) = blob_sides(&format!("{sha}^1"), sha, file);
        if !has_parent {
            old = Side::Empty;
        }
        self.diff_sides(&old, &new, opts)
    }

    /// Read one side's bytes.
    pub fn read_side(&self, side: &Side) -> Result<Vec<u8>> {
        match side {
            Side::Empty => Ok(Vec::new()),
            Side::Blob { rev, path } => {
                match self.git().args(["cat-file", "blob"]).arg(format!("{rev}:{path}")).run_bytes() {
                    Ok(b) => Ok(b),
                    // Path absent at that revision (e.g. staged-new file vs HEAD).
                    Err(crate::GitError::Failed { stderr, .. })
                        if stderr.contains("does not exist") || stderr.contains("Not a valid object") =>
                    {
                        Ok(Vec::new())
                    }
                    Err(e) => Err(e),
                }
            }
            Side::Worktree(path) => {
                let p = self.root().join(path);
                let meta = std::fs::symlink_metadata(&p).map_err(|e| crate::GitError::Io(format!("{path}: {e}")))?;
                if meta.file_type().is_symlink() {
                    let target = std::fs::read_link(&p).map_err(|e| crate::GitError::Io(e.to_string()))?;
                    Ok(target.to_string_lossy().into_owned().into_bytes())
                } else if meta.is_dir() {
                    // Submodule / nested repo: no textual content.
                    Ok(Vec::new())
                } else {
                    std::fs::read(&p).map_err(|e| crate::GitError::Io(format!("{path}: {e}")))
                }
            }
        }
    }

    /// Fetch both sides and diff them with the in-process engine.
    pub fn diff_sides(&self, old: &Side, new: &Side, opts: DiffOptions) -> Result<Diff> {
        let old_b = self.read_side(old)?;
        let new_b = self.read_side(new)?;
        if crate::engine::looks_binary(&old_b) || crate::engine::looks_binary(&new_b) {
            return Ok(Diff { is_binary: true, ..Default::default() });
        }
        let old_s = String::from_utf8_lossy(&old_b);
        let new_s = String::from_utf8_lossy(&new_b);
        let lossy = matches!(old_s, std::borrow::Cow::Owned(_)) || matches!(new_s, std::borrow::Cow::Owned(_));
        let mut diff = crate::engine::diff_texts(&old_s, &new_s, opts);
        if lossy {
            // Not valid UTF-8: viewable, but partial staging would corrupt
            // the bytes on write-back, so don't offer it.
            diff.source = None;
        }
        Ok(diff)
    }
}
