//! File actions shared by the file context menus: open, reveal, copy path.

use std::path::Path;

use gpui::{App, ClipboardItem};

/// `file:///a%20b/c` for `cx.open_url`.
pub fn file_url(path: &Path) -> String {
    url::Url::from_file_path(path).map(String::from).unwrap_or_default()
}

/// Reveal `root/rel` in the file manager; deleted files reveal their folder.
pub fn reveal(root: &Path, rel: &str, cx: &mut App) {
    let abs = root.join(rel);
    cx.reveal_path(&if abs.exists() { abs } else { abs.parent().unwrap_or(root).to_path_buf() });
}

pub fn copy_path(root: &Path, rel: &str, cx: &mut App) {
    cx.write_to_clipboard(ClipboardItem::new_string(root.join(rel).display().to_string()));
}
