//! Hunk expansion: show more unchanged lines around a hunk, like Desktop's
//! expand buttons in the hunk header gutter (`text-diff-expansion.ts`).
//!
//! The diff keeps both full texts ([`Diff::source`]), so expansion is a pure
//! function: it splits the flat rows back into hunks, pulls context lines
//! from the new text into the requested gap, merges hunks whose gap closed,
//! and re-flattens. Added/deleted rows (and their word emphasis) are copied
//! unchanged; only context rows and headers are new.

use crate::diff::{Diff, DiffHunk, DiffLine, DiffLineKind};
use crate::engine::split_lines;

/// Lines revealed per click, as in Desktop (`DefaultDiffExpansionStep`).
pub const EXPANSION_STEP: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expansion {
    /// Reveal up to `EXPANSION_STEP` lines above hunk `h`.
    Up(usize),
    /// Reveal up to `EXPANSION_STEP` lines below hunk `h`.
    Down(usize),
    /// Reveal the whole gap above hunk `h` (merging it with the previous one).
    All(usize),
}

/// Unchanged lines between hunk `h` and the previous hunk (or the file start).
pub fn gap_above(diff: &Diff, h: usize) -> usize {
    let Some(hunk) = diff.hunks.get(h) else { return 0 };
    let prev_end = h.checked_sub(1).map_or(0, |p| diff.hunks[p].new_start + diff.hunks[p].new_len);
    hunk.new_start.saturating_sub(prev_end)
}

/// Unchanged lines between hunk `h` and the next hunk (or the file end).
pub fn gap_below(diff: &Diff, h: usize) -> usize {
    let Some(hunk) = diff.hunks.get(h) else { return 0 };
    let end = hunk.new_start + hunk.new_len;
    match diff.hunks.get(h + 1) {
        Some(next) => next.new_start.saturating_sub(end),
        None => diff.source.as_ref().map_or(0, |s| split_lines(&s.new).len()).saturating_sub(end),
    }
}

/// Whether `diff` can be expanded at all (full texts known, not truncated).
pub fn expandable(diff: &Diff) -> bool {
    diff.source.is_some() && !diff.truncated && !diff.hunks.is_empty()
}

struct Hunk {
    meta: DiffHunk,
    rows: Vec<DiffLine>,
}

/// `diff` with `e` applied, or `None` if there is nothing to reveal.
pub fn expand(diff: &Diff, e: Expansion) -> Option<Diff> {
    if !expandable(diff) {
        return None;
    }
    let src = diff.source.as_ref()?;
    let new_lines = split_lines(&src.new);
    // Flat rows -> hunks.
    let mut hunks: Vec<Hunk> = diff
        .hunks
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let end = diff.hunks.get(i + 1).map_or(diff.lines.len(), |n| n.header_index);
            Hunk {
                meta: h.clone(),
                rows: diff.lines[h.header_index + 1..end].to_vec(),
            }
        })
        .collect();
    let (h, up, count) = match e {
        Expansion::Up(h) => (h, true, gap_above(diff, h).min(EXPANSION_STEP)),
        Expansion::All(h) => (h, true, gap_above(diff, h)),
        Expansion::Down(h) => (h, false, gap_below(diff, h).min(EXPANSION_STEP)),
    };
    if count == 0 || h >= hunks.len() {
        return None;
    }
    let hunk = &mut hunks[h];
    if up {
        let (os, ns) = (hunk.meta.old_start - count, hunk.meta.new_start - count);
        let rows: Vec<DiffLine> = (0..count).map(|i| DiffLine::new(DiffLineKind::Context, Some(os + i), Some(ns + i), new_lines[ns + i].text)).collect();
        hunk.rows.splice(0..0, rows);
        hunk.meta.old_start = os;
        hunk.meta.new_start = ns;
    } else {
        let (os, ns) = (hunk.meta.old_start + hunk.meta.old_len, hunk.meta.new_start + hunk.meta.new_len);
        for i in 0..count {
            let line = &new_lines[ns + i];
            hunk.rows.push(DiffLine::new(DiffLineKind::Context, Some(os + i), Some(ns + i), line.text));
            if !line.terminated {
                hunk.rows.push(DiffLine::no_newline());
            }
        }
    }
    hunk.meta.old_len += count;
    hunk.meta.new_len += count;
    // Merge hunks that now touch.
    let mut merged: Vec<Hunk> = Vec::with_capacity(hunks.len());
    for hunk in hunks {
        match merged.last_mut() {
            Some(prev) if prev.meta.new_start + prev.meta.new_len >= hunk.meta.new_start => {
                prev.meta.old_len = hunk.meta.old_start + hunk.meta.old_len - prev.meta.old_start;
                prev.meta.new_len = hunk.meta.new_start + hunk.meta.new_len - prev.meta.new_start;
                prev.rows.extend(hunk.rows);
            }
            _ => merged.push(hunk),
        }
    }
    // Re-flatten.
    let mut out = Diff { source: diff.source.clone(), added: diff.added, deleted: diff.deleted, ..Default::default() };
    for hunk in merged {
        let meta = DiffHunk { header_index: out.lines.len(), ..hunk.meta };
        out.lines.push(DiffLine::hunk(&meta));
        out.hunks.push(meta);
        out.lines.extend(hunk.rows);
    }
    out.longest_line = (0..out.lines.len()).max_by_key(|&i| out.lines[i].text.len()).unwrap_or(0);
    Some(out)
}

#[cfg(test)]
#[path = "tests/expand.test.rs"]
mod tests;
