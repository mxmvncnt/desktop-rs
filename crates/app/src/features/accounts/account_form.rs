//! Add / edit an account.

use gd_ui::{Button, Tabs, Theme};
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

use crate::components::FieldRow;
use super::model::{OAuthApp, Provider, normalize_host};
use super::state::{AccountForm as FormState, AccountInputs, AccountsStore};
use super::status_text::StatusText;

#[derive(IntoElement)]
pub struct AccountForm {
    pub form: FormState,
    pub inputs: AccountInputs,
    pub store: Entity<AccountsStore>,
}

impl AccountForm {

    fn provider_tabs(&self) -> impl IntoElement + use<> {
        let store = self.store.clone();
        let selected = Provider::ALL.iter().position(|p| *p == self.form.provider).unwrap_or(0);
        let tabs = Provider::ALL
            .iter()
            .fold(Tabs::new("account-provider", selected), |t, p| t.tab(p.label(), None))
            .on_select(move |ix, _, cx| store.update(cx, |this, cx| this.set_account_provider(Provider::ALL[ix], cx)));
        div().w_full().child(tabs)
    }

    fn host_field(&self) -> FieldRow {
        let hint = match self.form.provider {
            Provider::GitHubEnterprise => "github.example.com",
            Provider::Gitea => "gitea.example.com",
            Provider::Forgejo => "codeberg.org",
            Provider::Generic => "git.example.com",
            _ => "",
        };
        FieldRow::new("Host", "account-host", &self.inputs.host, hint)
    }

    /// OAuth providers: the sign-in button, or "Create token" when no app is available, and the
    /// "use your own OAuth app" disclosure.
    fn oauth_section(&self, app: Option<&OAuthApp>, theme: &Theme, cx: &App) -> impl IntoElement + use<> {
        let provider = self.form.provider;
        let caption = |t: String| div().text_size(px(12.0)).text_color(theme.text_faint).child(t);
        let (store, advanced_store) = (self.store.clone(), self.store.clone());
        let waiting = self.form.waiting.as_ref();
        let advanced = self.form.advanced;
        let status = match (&self.form.signed_in, waiting) {
            (Some(_), _) => Some(StatusText::ok("Signed in. Add the account to save the token.")),
            (None, Some(_)) => Some(StatusText::ok("Waiting for you to authorize in the browser...")),
            _ if self.form.has_token => Some(StatusText::ok("Signed in. Sign in again to replace the token.")),
            _ => None,
        };
        let host = normalize_host(self.inputs.host.read(cx).as_str());
        let token_page = match provider {
            Provider::GitLab => format!("https://{host}/-/user_settings/personal_access_tokens?name=desktop-rs&scopes=api"),
            _ => format!("https://{host}/settings/tokens/new?scopes=repo,workflow,read:user&description=desktop-rs"),
        };
        div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .children(app.is_none().then(|| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(
                        Button::new("account-create-token")
                            .primary()
                            .compact()
                            .label("Create token")
                            .disabled(host.is_empty())
                            .on_click(move |_, _, cx| cx.open_url(&token_page)),
                    )
                    .child(caption("Opens the token page. Paste the token below.".into()))
            }))
            .children((app.is_some() || advanced).then(|| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(
                        Button::new("account-sign-in")
                            .compact()
                            .label("Sign in with browser")
                            .disabled(waiting.is_some() || app.is_none())
                            .on_click(move |_, _, cx| store.update(cx, |this, cx| this.start_signin(cx))),
                    )
                    .children(status)
            }))
            .children(waiting.zip(provider.uses_device_flow().then_some(())).and_then(|(w, ())| {
                w.url.as_ref().map(|url| {
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(FieldRow::new("Your code", "account-device-code", &self.inputs.copyable, ""))
                        .child(caption(format!("Enter it at {url} (opened in your browser).")))
                })
            }))
            .child(div().flex().child(
                Button::new("account-advanced")
                    .ghost()
                    .compact()
                    .label(if advanced { "Advanced: use your own OAuth app (hide)" } else { "Advanced: use your own OAuth app" })
                    .on_click(move |_, _, cx| advanced_store.update(cx, |this, cx| this.toggle_advanced(cx))),
            ))
            .children(advanced.then(|| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .child(FieldRow::new("OAuth client ID", "account-client-id", &self.inputs.client_id, "Client ID of your OAuth app"))
                    .child(caption(if provider.uses_device_flow() {
                        format!("Register an OAuth app on {} and enable the device flow.", self.host_label())
                    } else {
                        format!("Register an OAuth app on {} (public client) with this redirect URI:", self.host_label())
                    }))
                    .children((!provider.uses_device_flow()).then(|| FieldRow::new("Redirect URI", "account-redirect-uri", &self.inputs.copyable, "")))
            }))
    }

    fn host_label(&self) -> String {
        match self.form.provider.fixed_host() {
            Some(host) => host.into(),
            None => "your host".into(),
        }
    }

    fn buttons(&self) -> impl IntoElement + use<> {
        let (save, cancel, remove) = (self.store.clone(), self.store.clone(), self.store.clone());
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                Button::new("account-save")
                    .primary()
                    .compact()
                    .label(if self.form.editing.is_some() { "Save" } else { "Add account" })
                    .on_click(move |_, _, cx| save.update(cx, |this, cx| this.save_account(cx))),
            )
            .child(
                Button::new("account-cancel")
                    .ghost()
                    .compact()
                    .label("Cancel")
                    .on_click(move |_, _, cx| cancel.update(cx, |this, cx| this.close_account_form(cx))),
            )
            .children(self.form.editing.is_some().then(|| {
                Button::new("account-remove")
                    .style(gd_ui::ButtonStyle::Danger)
                    .compact()
                    .label("Remove")
                    .on_click(move |_, _, cx| remove.update(cx, |this, cx| this.remove_account(cx)))
            }))
    }
}

impl RenderOnce for AccountForm {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let app = self.form.provider.oauth_app(&normalize_host(self.inputs.host.read(cx).as_str()), self.inputs.client_id.read(cx).as_str());
        let token_field = !self.form.provider.has_oauth() || app.is_none();
        let editing = self.form.editing.is_some();
        let inp = &self.inputs;
        let token_hint = if self.form.has_token { "Leave empty to keep the saved token" } else { "Personal access token" };
        let oauth = self.form.provider.has_oauth();
        div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(div().font_weight(gpui::FontWeight::MEDIUM).child(if editing { "Edit account" } else { "Add account" }))
            .child(self.provider_tabs())
            .child(FieldRow::new("Display name", "account-name", &inp.display_name, "Work"))
            .child(self.host_field())
            .children(oauth.then(|| self.oauth_section(app.as_ref(), &theme, cx)))
            .child(FieldRow::new("Username", "account-username", &inp.username, "octocat"))
            .children(token_field.then(|| FieldRow::new("Personal access token", "account-token", &inp.token, token_hint)))
            .children(token_field.then(|| {
                div().text_size(px(12.0)).text_color(theme.text_faint).child("The token is shown while typing and is not validated online.")
            }))
            .children(self.form.error.as_deref().map(StatusText::err))
            .child(self.buttons())
    }
}
