//! Add / edit an account.

use gd_ui::{Button, Icon, OptionRow, Theme, overlay_card};
use gpui::{App, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce, StatefulInteractiveElement, Styled, Window, deferred, div, px};

use crate::components::{BranchRow, FieldRow};
use super::model::{Provider, SignInMethod, normalize_host};
use super::state::{AccountForm as FormState, AccountInputs, AccountsStore};
use super::status_text::StatusText;

#[derive(IntoElement)]
pub struct AccountForm {
    pub form: FormState,
    pub inputs: AccountInputs,
    pub store: Entity<AccountsStore>,
}

/// Width of the account type control and of its menu: fits "GitHub Enterprise" and the chevron.
const SELECT_WIDTH: f32 = 220.0;
const SELECT_ROW: f32 = 28.0;

impl AccountForm {
    /// Dropdown of the providers. Like a native select, the open menu puts the current choice over the control.
    fn provider_select(&self, theme: &Theme, cx: &App) -> impl IntoElement + use<> {
        let provider = self.form.provider;
        let toggle = self.store.clone();
        let menu = self.form.provider_menu.then(|| {
            let rows = Provider::ALL.iter().enumerate().map(|(ix, p)| {
                let (store, p) = (self.store.clone(), *p);
                BranchRow::new("account-provider", ix, p.full_label(), move |_, cx| store.update(cx, |this, cx| this.set_account_provider(p, cx)))
                    .icon(Icon::Globe)
                    .height(SELECT_ROW)
                    .selected(p == provider)
            });
            let ix = Provider::ALL.iter().position(|p| *p == provider).unwrap_or(0);
            let close = self.store.clone();
            // The backdrop catches the click that dismisses the menu, wherever it lands (even on the control).
            deferred(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .child(
                        div()
                            .absolute()
                            .top(px(-3000.0))
                            .left(px(-3000.0))
                            .size(px(6000.0))
                            .occlude()
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| close.update(cx, |this, cx| this.toggle_provider_menu(cx))),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(-(2.0 + ix as f32 * SELECT_ROW)))
                            .left_0()
                            .w(px(SELECT_WIDTH))
                            .child(overlay_card(cx).occlude().p(px(4.0)).children(rows)),
                    ),
            )
            .priority(2)
        });
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(div().text_size(px(12.0)).text_color(theme.text_muted).child("Account type"))
            .child(
                div().relative().w(px(SELECT_WIDTH)).child(
                    // Styled like the text fields: the value on the left, the chevron on the right.
                    div()
                        .id("account-provider")
                        .flex()
                        .items_center()
                        .justify_between()
                        .h(px(32.0))
                        .px(px(10.0))
                        .rounded(px(gd_ui::theme::CONTROL_RADIUS + 1.0))
                        .bg(theme.input_bg)
                        .border_1()
                        .border_color(theme.border)
                        .cursor_pointer()
                        .hover_style(|s| s.border_color(theme.text_faint))
                        .child(provider.full_label())
                        .child(gd_ui::icon(Icon::ChevronDown).size(px(14.0)).text_color(theme.text_muted))
                        .on_click(move |_, _, cx| toggle.update(cx, |this, cx| this.toggle_provider_menu(cx))),
                )
                .children(menu),
            )
    }

    fn host_field(&self) -> FieldRow {
        let hint = match self.form.provider {
            Provider::GitHubEnterprise => "github.example.com",
            Provider::Gitea => "gitea.example.com",
            Provider::Forgejo => "codeberg.org",
            _ => "git.example.com",
        };
        FieldRow::new("Host", "account-host", &self.inputs.host, hint)
    }

    /// The radio group and the body of the chosen method.
    fn sign_in(&self, method: SignInMethod, browser: bool, theme: &Theme, cx: &App) -> impl IntoElement + use<> {
        let methods = [(SignInMethod::Browser, "Browser (recommended)"), (SignInMethod::Token, "Personal access token"), (SignInMethod::OwnApp, "Your own OAuth app")];
        let rows = methods.into_iter().filter(|(m, _)| browser || *m != SignInMethod::Browser).map(|(m, title)| {
            let store = self.store.clone();
            OptionRow::new(title, m == method, title, move |_, cx| store.update(cx, |this, cx| this.set_sign_in_method(m, cx)))
        });
        let body = match method {
            SignInMethod::Browser => self.browser_sign_in(false, theme, cx).into_any_element(),
            SignInMethod::Token => self.token_body(theme, cx).into_any_element(),
            SignInMethod::OwnApp => self.own_app_body(theme, cx).into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(div().text_size(px(12.0)).text_color(theme.text_muted).child("Sign in with"))
            .children(rows)
            .child(div().mt(px(4.0)).child(body))
    }

    /// "Sign in with browser" and, while the browser is open, the device code and Cancel.
    /// `needs_client_id`: the own-app method, which can't start without an id.
    fn browser_sign_in(&self, needs_client_id: bool, theme: &Theme, cx: &App) -> impl IntoElement + use<> {
        let caption = |t: String| div().text_size(px(12.0)).text_color(theme.text_faint).child(t);
        let (start, cancel) = (self.store.clone(), self.store.clone());
        let waiting = self.form.waiting.as_ref();
        let missing_id = needs_client_id && self.inputs.client_id.read(cx).as_str().trim().is_empty();
        let signed_in = self.form.signed_in.is_some();
        let username = self.inputs.username.read(cx).as_str().to_string();
        let status = match waiting {
            Some(_) => Some(StatusText::ok("Waiting for the browser...")),
            None if signed_in => Some(StatusText::ok(format!("Signed in as {username}. Add the account to save the token."))),
            None if self.form.has_token => Some(StatusText::ok("Signed in. Sign in again to replace the token.")),
            None => None,
        };
        let button = Button::new("account-sign-in")
            .compact()
            .label(if signed_in { "Sign in again" } else { "Sign in with browser" })
            .disabled(missing_id)
            .on_click(move |_, _, cx| start.update(cx, |this, cx| this.start_signin(cx)));
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(match waiting {
                        Some(_) => Button::new("account-sign-in-cancel")
                            .ghost()
                            .compact()
                            .label("Cancel")
                            .on_click(move |_, _, cx| cancel.update(cx, |this, cx| this.cancel_signin(cx))),
                        None if signed_in || missing_id => button,
                        None => button.primary(),
                    })
                    .children(status),
            )
            .children(missing_id.then(|| caption("Enter the client ID of your OAuth app to sign in.".into())))
            .children(waiting.zip(self.form.provider.uses_device_flow().then_some(())).and_then(|(w, ())| {
                w.url.as_ref().map(|url| {
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(FieldRow::new("Your code", "account-device-code", &self.inputs.copyable, ""))
                        .child(caption(format!("Enter it at {url} (opened in your browser).")))
                })
            }))
    }

    fn token_body(&self, theme: &Theme, cx: &App) -> impl IntoElement + use<> {
        let host = normalize_host(self.inputs.host.read(cx).as_str());
        let token_page = match self.form.provider {
            Provider::GitLab => format!("https://{host}/-/user_settings/personal_access_tokens?name=desktop-rs&scopes=api"),
            _ => format!("https://{host}/settings/tokens/new?scopes=repo,workflow,read:user&description=desktop-rs"),
        };
        let hint = if self.form.has_token { "Leave empty to keep the saved token" } else { "Paste the token" };
        div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(
                        Button::new("account-create-token")
                            .compact()
                            .label("Create token")
                            .disabled(host.is_empty())
                            .on_click(move |_, _, cx| cx.open_url(&token_page)),
                    )
                    .child(div().text_size(px(12.0)).text_color(theme.text_faint).child("Opens the token page. Paste the token below.")),
            )
            .child(FieldRow::new("Token", "account-token", &self.inputs.token, hint))
    }

    fn own_app_body(&self, theme: &Theme, cx: &App) -> impl IntoElement + use<> {
        let provider = self.form.provider;
        let host = self.form.provider.fixed_host().map_or("your host".to_string(), Into::into);
        let caption = |t: String| div().text_size(px(12.0)).text_color(theme.text_faint).child(t);
        div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(FieldRow::new("OAuth client ID", "account-client-id", &self.inputs.client_id, "Client ID of your OAuth app"))
            .child(caption(if provider.uses_device_flow() {
                format!("Register an OAuth app on {host} and enable the device flow.")
            } else {
                format!("Register an OAuth app on {host} (public client) with this redirect URI:")
            }))
            .children((!provider.uses_device_flow()).then(|| FieldRow::new("Redirect URI", "account-redirect-uri", &self.inputs.copyable, "")))
            .child(self.browser_sign_in(true, theme, cx))
    }

    /// Bitbucket and Generic: git needs a username and there is no `/user` lookup.
    fn manual_credentials(&self) -> impl IntoElement + use<> {
        let inp = &self.inputs;
        let hint = if self.form.has_token { "Leave empty to keep the saved token" } else { "Personal access token" };
        div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(FieldRow::new("Username", "account-username", &inp.username, "octocat"))
            .child(FieldRow::new("Personal access token", "account-token", &inp.token, hint))
    }

    fn buttons(&self, method: SignInMethod) -> impl IntoElement + use<> {
        let (save, cancel, remove) = (self.store.clone(), self.store.clone(), self.store.clone());
        let form = &self.form;
        let editing = form.editing.is_some();
        // A browser sign-in has its own button; saving makes sense once it finished.
        let can_save = editing || method == SignInMethod::Token || !form.provider.has_oauth() || form.signed_in.is_some();
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .children(can_save.then(|| {
                Button::new("account-save")
                    .primary()
                    .compact()
                    .label(match (editing, form.checking) {
                        (_, true) => "Checking...",
                        (true, _) => "Save",
                        _ => "Add account",
                    })
                    .disabled(form.checking)
                    .on_click(move |_, _, cx| save.update(cx, |this, cx| this.save_account(cx)))
            }))
            .child(
                Button::new("account-cancel")
                    .ghost()
                    .compact()
                    .label("Cancel")
                    .on_click(move |_, _, cx| cancel.update(cx, |this, cx| this.close_account_form(cx))),
            )
            .children(editing.then(|| {
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
        let provider = self.form.provider;
        let browser = provider.oauth_app(&normalize_host(self.inputs.host.read(cx).as_str()), "").is_some();
        // The host can change what is available (GitHub Enterprise has no shared app).
        let method = if self.form.method == SignInMethod::Browser && !browser { SignInMethod::Token } else { self.form.method };
        let editing = self.form.editing.is_some();
        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(div().font_weight(gpui::FontWeight::MEDIUM).child(if editing { "Edit account" } else { "Add account" }))
            .child(self.provider_select(&theme, cx))
            .children(provider.fixed_host().is_none().then(|| self.host_field()))
            .children(editing.then(|| FieldRow::new("Display name", "account-name", &self.inputs.display_name, "Work")))
            .child(if provider.has_oauth() { self.sign_in(method, browser, &theme, cx).into_any_element() } else { self.manual_credentials().into_any_element() })
            .children(self.form.error.as_deref().map(StatusText::err))
            .child(self.buttons(method))
    }
}
