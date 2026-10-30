//! Text selection in the commit header: line 0 is the summary, line 1 the description. Reuses the
//! diff's `TextSel` model; `texts` ties the selection to the commit it was made on.

use gpui::{ClipboardItem, Context, Window};

use crate::features::diff::text_selection::model::{TextPos, TextSel, TextSide};
use crate::workspace::Workspace;

#[derive(Clone, Default)]
pub struct CommitSelection {
    pub sel: Option<TextSel>,
    /// Summary and description the selection applies to.
    pub texts: [String; 2],
    pub drag: bool,
}

impl CommitSelection {
    /// The selection if it was made on these texts.
    pub fn on(&self, texts: &[String; 2]) -> Option<TextSel> {
        self.sel.filter(|_| &self.texts == texts)
    }
}

impl Workspace {
    /// Mouse down on the summary (`line` 0) or description (`line` 1); same gestures as the diff.
    pub fn commit_text_down(
        &mut self,
        texts: [String; 2],
        pos: TextPos,
        clicks: usize,
        shift: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus(window, cx);
        let prev = self.commit_sel.on(&texts);
        let sel = TextSel::press(prev, pos, TextSide::Unified, clicks, shift, &texts[pos.line]);
        self.diff_view.text_sel = None;
        self.commit_sel = CommitSelection { sel: Some(sel), texts, drag: clicks == 1 };
        cx.notify();
    }

    pub fn commit_text_drag(&mut self, pos: TextPos, cx: &mut Context<Self>) {
        let Some(sel) = self.commit_sel.sel.as_mut().filter(|s| s.head != pos) else { return };
        sel.head = pos;
        cx.notify();
    }

    pub fn copy_commit_text(&mut self, cx: &mut Context<Self>) {
        let CommitSelection { sel: Some(sel), texts, .. } = &self.commit_sel else { return };
        let (a, b) = sel.ordered();
        let text = (a.line..=b.line)
            .filter_map(|l| sel.cols_in(l, TextSide::Unified, texts[l].len()).map(|r| &texts[l][r]))
            .collect::<Vec<_>>()
            .join("\n");
        if !text.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }
}
