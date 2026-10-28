//! Snapshot views: themes and settings pages, set directly (no settings file is written).

use gpui::{Context, Window};

use super::{SettingsPage, ThemeMode};
use crate::snapshot::View;
use crate::workspace::{Overlay, Workspace};

pub fn views() -> &'static [View] {
    &[
        View { name: "theme:*", steps: &[theme] },
        View { name: "settings", steps: &[page] },
        View { name: "settings-light", steps: &[page] },
        View { name: "settings-diff", steps: &[page] },
        View { name: "changes-light", steps: &[page] },
    ]
}

/// `theme:<id>:<dark|light>[:settings]`: a color theme, optionally with the settings modal.
fn theme(ws: &mut Workspace, view: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    let mut parts = view.split(':').skip(1);
    ws.settings.color_theme = parts.next().unwrap_or_default().to_string();
    ws.settings.theme = if parts.next() == Some("light") { ThemeMode::Light } else { ThemeMode::Dark };
    let settings = parts.next() == Some("settings");
    if settings {
        ws.overlays.push(Overlay::Settings);
    }
    if let (false, Some(r)) = (settings, &ws.repo) {
        r.update(cx, |r, cx| r.select_change(1, cx)); // second file: Rust, for syntax colors
    }
    ws.apply_theme(window, cx);
}

fn page(ws: &mut Workspace, view: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.settings.theme = if view.ends_with("light") { ThemeMode::Light } else { ThemeMode::Dark };
    ws.apply_theme(window, cx);
    if view.starts_with("settings") {
        ws.overlays.push(Overlay::Settings);
    }
    if view == "settings-diff" {
        ws.settings_page = SettingsPage::Diff;
    }
}
