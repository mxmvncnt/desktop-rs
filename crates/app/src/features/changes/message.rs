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

    /// gpui-ce caches the input's layout by text version, not by placeholder,
    /// so an empty box would keep painting the previous placeholder (e.g. the
    /// last commit's "Update x"). Re-emplacing "" bumps the version.
    pub(crate) fn sync_message_placeholder(&mut self, cx: &mut Context<Self>) {
        let Some(repo) = &self.repo else { return };
        let placeholder = repo.read(cx).default_summary(self.amending(cx));
        if placeholder == self.message_placeholder {
            return;
        }
        self.message_placeholder = placeholder;
        if self.message.read(cx).as_str().is_empty() {
            self.message.update(cx, |s, cx| s.emplace("", cx));
        }
    }
}
