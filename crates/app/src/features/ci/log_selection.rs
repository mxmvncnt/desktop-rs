//! Text selection in the job log, on the diff's `TextSel` model: one side (`Unified`), lines of `Log::lines`.
//! Pressing, dragging and select-all come from `log_view`, copy from the workspace's Ctrl/Cmd+C.

use gpui::{App, ClipboardItem, Context};

use super::state::CiStore;
use crate::features::diff::text_selection::model::{TextPos, TextSel, TextSide};

impl CiStore {
    /// Mouse down on the text of `pos.line`: a click starts a drag selection, shift extends it,
    /// double-click selects a word, triple-click the line.
    pub fn log_press(&mut self, pos: TextPos, clicks: usize, shift: bool, cx: &mut Context<Self>) {
        let Some(log) = &self.log.log else { return };
        self.log.sel = Some(TextSel::press(
            self.log.sel,
            pos,
            TextSide::Unified,
            clicks,
            shift,
            &log.lines[pos.line].text,
        ));
        self.log.dragging = clicks == 1;
        cx.notify();
    }

    /// The pointer moved to `pos` while the button is down.
    pub fn log_drag(&mut self, pos: TextPos, cx: &mut Context<Self>) {
        let Some(sel) = self.log.sel.as_mut().filter(|s| s.head != pos) else {
            return;
        };
        sel.head = pos;
        cx.notify();
    }

    /// Select every visible line.
    pub fn select_all_log(&mut self, cx: &mut Context<Self>) {
        let (Some(log), Some(&first), Some(&last)) = (
            &self.log.log,
            self.log.visible.first(),
            self.log.visible.last(),
        ) else {
            return;
        };
        let head = TextPos {
            line: last,
            col: log.lines[last].text.len(),
        };
        self.log.sel = Some(TextSel {
            anchor: TextPos {
                line: first,
                col: 0,
            },
            head,
            side: TextSide::Unified,
        });
        cx.notify();
    }

    /// The selected text: the visible lines of the selection (not the ones a collapsed section hides), one per row.
    pub fn log_selected_text(&self) -> String {
        let (Some(sel), Some(log)) = (self.log.sel, &self.log.log) else {
            return String::new();
        };
        let (start, end) = sel.ordered();
        let lines = self
            .log
            .visible
            .iter()
            .filter(|line| (start.line..=end.line).contains(line));
        let lines = lines.map(|&line| {
            let text = &log.lines[line].text;
            text.get(
                sel.cols_in(line, TextSide::Unified, text.len())
                    .unwrap_or(0..0),
            )
            .unwrap_or("")
        });
        lines.collect::<Vec<_>>().join("\n")
    }

    pub fn copy_log_selection(&self, cx: &App) {
        let text = self.log_selected_text();
        if !text.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }
}

#[cfg(test)]
#[path = "tests/log_selection.test.rs"]
mod tests;
