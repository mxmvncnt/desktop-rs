//! Text selection model of the diff, in (line index, byte offset) coordinates
//! so it survives virtualization. Pure logic, no UI.

use std::ops::Range;

use gd_git::{Diff, DiffLineKind};

use crate::features::diff::list::{NONE, SplitDiff};

/// A caret position: a line of `Diff::lines` and a byte offset into its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextPos {
    pub line: usize,
    pub col: usize,
}

/// Which text column a selection lives in. Split view selects within one side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextSide {
    Unified,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextSel {
    pub anchor: TextPos,
    pub head: TextPos,
    pub side: TextSide,
}

impl TextSel {
    /// `(start, end)` in document order.
    pub fn ordered(&self) -> (TextPos, TextPos) {
        (self.anchor.min(self.head), self.anchor.max(self.head))
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Selected byte range of `line` (whose text is `len` bytes) when shown on `side`.
    pub fn cols_in(&self, line: usize, side: TextSide, len: usize) -> Option<Range<usize>> {
        if side != self.side || self.is_empty() {
            return None;
        }
        let (a, b) = self.ordered();
        if line < a.line || line > b.line {
            return None;
        }
        let start = if line == a.line { a.col.min(len) } else { 0 };
        let end = if line == b.line { b.col.min(len) } else { len };
        (start < end).then_some(start..end)
    }

    /// Everything selectable on `side`.
    pub fn all(diff: &Diff, side: TextSide) -> Option<Self> {
        let first = diff.lines.iter().position(is_code)?;
        let last = diff.lines.iter().rposition(is_code)?;
        let head = TextPos { line: last, col: diff.lines[last].text.len() };
        Some(Self { anchor: TextPos { line: first, col: 0 }, head, side })
    }

    /// The selected text: code lines only, joined with newlines.
    pub fn text(&self, diff: &Diff, split: Option<&SplitDiff>) -> String {
        let (a, b) = self.ordered();
        let mut parts: Vec<&str> = Vec::new();
        let mut push = |line: usize| {
            let text = &diff.lines[line].text;
            let r = self.cols_in(line, self.side, text.len()).unwrap_or(0..0);
            parts.push(text.get(r).unwrap_or(""));
        };
        match (self.side, split) {
            (TextSide::Unified, _) | (_, None) => {
                for line in a.line..=b.line.min(diff.lines.len().saturating_sub(1)) {
                    if is_code(&diff.lines[line]) {
                        push(line);
                    }
                }
            }
            (side, Some(split)) => {
                for row in &split.rows {
                    let ix = if side == TextSide::Left { row.left } else { row.right };
                    let ix = ix as usize;
                    if ix != NONE as usize && (a.line..=b.line).contains(&ix) && is_code(&diff.lines[ix]) {
                        push(ix);
                    }
                }
            }
        }
        parts.join("\n")
    }
}

/// Lines that carry code text (not hunk headers or "no newline" markers).
pub fn is_code(line: &gd_git::DiffLine) -> bool {
    !matches!(line.kind, DiffLineKind::Hunk | DiffLineKind::NoNewline)
}

/// Byte range of the word (or run of same-class characters) around `col`.
pub fn word_at(text: &str, col: usize) -> Range<usize> {
    let class = |c: char| (c.is_alphanumeric() || c == '_', c.is_whitespace());
    let col = col.min(text.len());
    let before = text[..col].chars().next_back();
    let after = text[col..].chars().next();
    let Some(pivot) = after.or(before) else { return col..col };
    let k = class(pivot);
    let start = col - text[..col].chars().rev().take_while(|&c| class(c) == k).map(char::len_utf8).sum::<usize>();
    let end = col + text[col..].chars().take_while(|&c| class(c) == k).map(char::len_utf8).sum::<usize>();
    start..end
}

#[cfg(test)]
#[path = "tests/model.test.rs"]
mod tests;
