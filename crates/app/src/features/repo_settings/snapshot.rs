//! Snapshot views: the repository settings modal (`repo-settings[-<page>]`) and the right-click menu.

use gpui::{Context, Window, point, px};

use super::state::RepoSettingsPage;
use crate::features::settings::ThemeMode;
use crate::snapshot::View;
use crate::workspace::Workspace;

pub fn views() -> &'static [View] {
    &[
        View { name: "repo-settings*", steps: &[open] },
        View { name: "repo-menu-context", steps: &[menu] },
    ]
}

fn open(ws: &mut Workspace, view: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.settings.theme = ThemeMode::Dark;
    ws.apply_theme(window, cx);
    ws.open_repo_settings(cx);
    let page = match view.strip_prefix("repo-settings-") {
        Some("identity") => RepoSettingsPage::Identity,
        Some("account") => RepoSettingsPage::Account,
        _ => RepoSettingsPage::Info,
    };
    ws.set_repo_settings_page(page, cx);
}

fn menu(ws: &mut Workspace, _: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.open_repo_menu(point(px(120.0), px(40.0)), window, cx);
}
