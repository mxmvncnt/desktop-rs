//! Add / edit an account.

use gd_ui::{Button, Tabs, Theme};
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

use super::field_row::FieldRow;
use super::model::Provider;
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

    /// OAuth providers: client id (when not built in), what to register, the sign-in button and its state.
    fn oauth_section(&self, theme: &Theme) -> impl IntoElement + use<> {
        let provider = self.form.provider;
        let caption = |t: String| div().text_size(px(12.0)).text_color(theme.text_faint).child(t);
        let store = self.store.clone();
        let waiting = self.form.waiting.as_ref();
        let needs_client_id = provider.builtin_client_id().is_none();
        let status = match (&self.form.signed_in, waiting) {
            (Some(_), _) => Some(StatusText::ok("Signed in. Add the account to save the token.")),
            (None, Some(_)) => Some(StatusText::ok("Waiting for you to authorize in the browser...")),
            _ if self.form.has_token => Some(StatusText::ok("Signed in. Sign in again to replace the token.")),
            _ => None,
        };
        div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .children(needs_client_id.then(|| FieldRow::new("OAuth client ID", "account-client-id", &self.inputs.client_id, "Client ID of your OAuth app")))
            .children((needs_client_id && !provider.uses_device_flow()).then(|| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(caption(format!("Register an OAuth app on {} (public client) with this redirect URI:", self.host_label())))
                    .child(FieldRow::new("Redirect URI", "account-redirect-uri", &self.inputs.copyable, ""))
            }))
            .children((needs_client_id && provider.uses_device_flow()).then(|| {
                caption(format!("Register an OAuth app on {} and enable the device flow.", self.host_label()))
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
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(
                        Button::new("account-sign-in")
                            .compact()
                            .label("Sign in with browser")
                            .disabled(waiting.is_some())
                            .on_click(move |_, _, cx| store.update(cx, |this, cx| this.start_signin(cx))),
                    )
                    .children(status),
            )
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
            .children(oauth.then(|| self.oauth_section(&theme)))
            .child(FieldRow::new("Username", "account-username", &inp.username, "octocat"))
            .children((!oauth).then(|| FieldRow::new("Personal access token", "account-token", &inp.token, token_hint)))
            .children((!oauth).then(|| {
                div().text_size(px(12.0)).text_color(theme.text_faint).child("The token is shown while typing and is not validated online.")
            }))
            .children(self.form.error.as_deref().map(StatusText::err))
            .child(self.buttons())
    }
}
