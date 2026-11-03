//! Snapshot views: the repository settings modal (`repo-settings[-<page>]`) and the right-click menu.

use gpui::{Context, Window, point, px};

use super::state::RepoSettingsPage;
use crate::features::settings::ThemeMode;
use crate::snapshot::View;
use crate::workspace::Workspace;

pub fn views() -> &'static [View] {
    &[
        View {
            name: "repo-settings-remotes-edit",
            steps: &[open, edit_remote],
        },
        View {
            name: "repo-settings-remotes-remove",
            steps: &[open, remove_remote],
        },
        View {
            name: "repo-settings-config-edit",
            steps: &[open, edit_config],
        },
        View {
            name: "repo-settings*",
            steps: &[open],
        },
        View {
            name: "repo-menu-context",
            steps: &[menu],
        },
    ]
}

fn open(ws: &mut Workspace, view: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.settings.theme = ThemeMode::Dark;
    ws.apply_theme(window, cx);
    ws.open_repo_settings(cx);
    let page = match view.strip_prefix("repo-settings-") {
        Some("remotes" | "remotes-edit" | "remotes-remove") => RepoSettingsPage::Remotes,
        Some("ignore") => RepoSettingsPage::Ignore,
        Some("config" | "config-edit") => RepoSettingsPage::Config,
        Some("identity") => RepoSettingsPage::Identity,
        Some("account") => RepoSettingsPage::Account,
        _ => RepoSettingsPage::Info,
    };
    ws.set_repo_settings_page(page, cx);
}

/// Second step (the remotes are loaded by now): the edit form / remove confirmation of `upstream`.
fn edit_remote(ws: &mut Workspace, _: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    ws.repo_settings
        .update(cx, |s, cx| s.open_remote_form(Some("upstream".into()), cx));
}

fn remove_remote(ws: &mut Workspace, _: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    ws.repo_settings
        .update(cx, |s, cx| s.confirm_remove_remote("upstream".into(), cx));
}

fn edit_config(ws: &mut Workspace, _: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    let entry = ws
        .repo_settings
        .read(cx)
        .config
        .iter()
        .find(|e| e.key == "remote.origin.fetch")
        .cloned();
    ws.repo_settings
        .update(cx, |s, cx| s.open_config_form(entry, cx));
}

fn menu(ws: &mut Workspace, _: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.open_repo_menu(point(px(120.0), px(40.0)), window, cx);
}
