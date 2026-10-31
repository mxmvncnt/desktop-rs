//! Snapshot views: Accounts settings with sample data (nothing is read or written).

use gpui::{App, Context, Entity, Window};
use gpui_elements::editable_text::EditableTextState;

use super::model::{Account, Provider};
use super::state::{AccountForm, Waiting};
use super::status_text::StatusText;
use crate::features::settings::{SettingsPage, ThemeMode};
use crate::snapshot::View;
use crate::workspace::{Overlay, Workspace};

pub fn views() -> &'static [View] {
    &[View { name: "settings-accounts*", steps: &[accounts_sample] }]
}

fn accounts_sample(ws: &mut Workspace, view: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.settings.theme = if view.ends_with("light") { ThemeMode::Light } else { ThemeMode::Dark };
    ws.apply_theme(window, cx);
    if view.contains("-norepo") {
        ws.repo = None;
    }
    ws.overlays.push(Overlay::Settings);
    ws.settings_page = SettingsPage::Accounts;
    let acct = |id: &str, provider, name: &str, host: &str, user: &str, token: &str| Account {
        id: id.into(),
        provider,
        name: name.into(),
        host: host.into(),
        username: user.into(),
        oauth: false,
        token: token.into(),
    };
    if view.ends_with("plaintext") {
        super::secrets::PLAINTEXT.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    let editing = view.ends_with("edit");
    let waiting = view.ends_with("oauth-waiting");
    let selfhosted = view.ends_with("oauth-selfhosted");
    let show_form = view.ends_with("form") || editing || waiting || selfhosted;
    ws.accounts.update(cx, |ui, cx| {
        ui.accounts = vec![
            acct("1", Provider::GitHub, "Personal", "github.com", "octocat", "ghp_x"),
            acct("2", Provider::GitHubEnterprise, "Work", "github.corp.example.com", "mvincent", "ghp_y"),
            acct("3", Provider::GitLab, "GitLab", "gitlab.com", "", ""),
        ];
        let inp = &ui.inputs;
        let set = |input: &Entity<EditableTextState>, t: &str, cx: &mut App| input.update(cx, |s, cx| s.emplace(t, cx));
        set(&inp.global_name, "Maxime Vincent", cx);
        set(&inp.global_email, "maxime@example.com", cx);
        set(&inp.local_name, "Max (work)", cx);
        set(&inp.local_email, "max@corp.example.com", cx);
        let global = gd_git::Identity { name: "Maxime Vincent".into(), email: "maxime@example.com".into() };
        let local = gd_git::Identity { name: "Max (work)".into(), email: "max@corp.example.com".into() };
        ui.identity.global = global;
        ui.identity.effective = Some(gd_git::EffectiveIdentity { identity: local.clone(), source: gd_git::IdentitySource::Repository });
        ui.identity.local = local;
        ui.identity.message = Some(StatusText::ok("Saved to your global git config"));
        if show_form {
            set(&inp.display_name, if editing { "Work" } else { "" }, cx);
            let host = if editing {
                "github.corp.example.com"
            } else if selfhosted {
                "gitea.example.com"
            } else {
                "github.com"
            };
            set(&inp.host, host, cx);
            set(&inp.copyable, if waiting { "WDJB-MJHT" } else { super::oauth::REDIRECT_URI }, cx);
            let provider = match (editing, selfhosted) {
                (true, _) => Provider::GitHubEnterprise,
                (_, true) => Provider::Gitea,
                _ => Provider::GitHub,
            };
            set(&inp.username, if editing { "mvincent" } else { "" }, cx);
            ui.form = Some(AccountForm {
                editing: editing.then(|| "2".to_string()),
                provider,
                has_token: editing,
                error: None,
                waiting: waiting.then(|| Waiting { cancel: Default::default(), url: Some("https://github.com/login/device".into()) }),
                signed_in: None,
            });
        }
        cx.notify();
    });
    cx.notify();
}
