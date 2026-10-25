//! The commit box holds one message: first line is the summary, the rest the description.

use gpui::Context;

use crate::workspace::Workspace;

/// Split a message into (summary, description).
pub fn split(text: &str) -> (String, String) {
    let (first, rest) = text.split_once('\n').unwrap_or((text, ""));
    (first.trim().to_string(), rest.trim().to_string())
}

pub fn join(summary: &str, description: &str) -> String {
    if description.is_empty() { summary.to_string() } else { format!("{summary}\n\n{description}") }
}

impl Workspace {
    pub fn commit_message(&self, cx: &gpui::App) -> (String, String) {
        split(self.message.read(cx).as_str())
    }

    pub fn set_commit_message(&mut self, summary: &str, description: &str, cx: &mut Context<Self>) {
        let text = join(summary, description);
        self.message.update(cx, |s, cx| s.emplace(&text, cx));
    }
}
