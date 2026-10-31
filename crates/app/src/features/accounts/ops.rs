//! `impl AccountsStore`: git identity load/save (background `git config`) and
//! account add/edit/remove.

use gd_git::{Identity, Repository};
use gpui::{AppContext as _, Context, Entity};
use gpui_elements::editable_text::EditableTextState;

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::model::{self, Account, Provider, normalize_host};
use super::oauth::{self, Progress};
use super::repo_account;
use super::secrets;
use super::state::{AccountForm, AccountsStore, Waiting, register_credentials};
use super::status_text::StatusText;
use super::token::TokenSecret;
use crate::repo::spawn_load;

/// Global identity, the open repository's (local, effective) identity, and its remote host.
type IdentityLoad = (Identity, Option<(Identity, gd_git::EffectiveIdentity)>, Option<String>);

fn input_text(input: &Entity<EditableTextState>, cx: &Context<AccountsStore>) -> String {
    input.read(cx).as_str().trim().to_string()
}

fn set_input(input: &Entity<EditableTextState>, text: &str, cx: &mut Context<AccountsStore>) {
    input.update(cx, |s, cx| s.emplace(text, cx));
}

impl AccountsStore {
    /// Read the global (and, with a repo open, local + effective) identity off the UI thread.
    pub fn load_identity(&mut self, repo: Option<Repository>, cx: &mut Context<Self>) {
        spawn_load(
            self,
            |this| &mut this.identity_load,
            move || {
                let global = gd_git::global_identity()?;
                let repo_part = match repo {
                    Some(ref r) => Some((r.local_identity()?, r.effective_identity()?)),
                    None => None,
                };
                let remote_host = repo.as_ref().and_then(Repository::remote_host);
                Ok::<_, gd_git::GitError>((global, repo_part, remote_host))
            },
            |this, res, cx| this.apply_identity(res, cx),
            cx,
        );
    }

    fn apply_identity(
        &mut self,
        res: Result<IdentityLoad, gd_git::GitError>,
        cx: &mut Context<Self>,
    ) {
        match res {
            Ok((global, repo_part, remote_host)) => {
                let id = &mut self.identity;
                id.remote_host = remote_host;
                id.global = global;
                (id.local, id.effective) = match repo_part {
                    Some((l, e)) => (l, Some(e)),
                    None => (Identity::default(), None),
                };
                let (g, l) = (id.global.clone(), id.local.clone());
                let inp = &self.inputs;
                for (input, text) in [
                    (&inp.global_name, g.name),
                    (&inp.global_email, g.email),
                    (&inp.local_name, l.name),
                    (&inp.local_email, l.email),
                ] {
                    set_input(input, &text, cx);
                }
            }
            Err(e) => self.identity.message = Some(StatusText::err(e.to_string())),
        }
        cx.notify();
    }

    pub(super) fn save_global_identity(&mut self, cx: &mut Context<Self>) {
        let inp = &self.inputs;
        let id = Identity { name: input_text(&inp.global_name, cx), email: input_text(&inp.global_email, cx) };
        self.write_identity(None, None, id, "Saved to your global git config", cx);
    }

    pub(super) fn save_local_identity(&mut self, repo: Repository, cx: &mut Context<Self>) {
        let inp = &self.inputs;
        let id = Identity { name: input_text(&inp.local_name, cx), email: input_text(&inp.local_email, cx) };
        self.write_identity(Some(repo.clone()), Some(repo), id, "Saved to this repository's config", cx);
    }

    pub(super) fn clear_local_identity(&mut self, repo: Repository, cx: &mut Context<Self>) {
        self.write_identity(Some(repo.clone()), Some(repo), Identity::default(), "Override removed", cx);
    }

    /// Write `id` to `target` (or the global config when `None`), then reload for the `open` repository.
    fn write_identity(
        &mut self,
        target: Option<Repository>,
        open: Option<Repository>,
        id: Identity,
        done: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.identity.message = None;
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_spawn(async move {
                    match target {
                        Some(r) => r.set_local_identity(&id),
                        None => gd_git::set_global_identity(&id),
                    }
                })
                .await;
            this.update(cx, |this, cx| {
                this.load_identity(open, cx);
                this.identity.message = Some(match res {
                    Ok(()) => StatusText::ok(done),
                    Err(e) => StatusText::err(e.to_string()),
                });
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Pick the account of a repository (`None` = automatic) and save the pick.
    pub(super) fn set_repo_account(&mut self, repo_path: &Path, id: Option<String>, cx: &mut Context<Self>) {
        let key = repo_path.to_string_lossy().into_owned();
        match id {
            Some(id) => self.repo_accounts.insert(key, id),
            None => self.repo_accounts.remove(&key),
        };
        repo_account::save(&self.repo_accounts);
        cx.notify();
    }

    // ---- accounts ----------------------------------------------------------

    pub(super) fn open_account_form(&mut self, editing: Option<String>, cx: &mut Context<Self>) {
        let acct = editing.as_ref().and_then(|id| self.accounts.iter().find(|a| &a.id == id)).cloned();
        let provider = acct.as_ref().map_or(Provider::GitHub, |a| a.provider);
        let a = acct.clone().unwrap_or(Account {
            id: String::new(),
            provider,
            name: String::new(),
            host: provider.fixed_host().unwrap_or_default().into(),
            username: String::new(),
            oauth: false,
            token: String::new(),
        });
        let inp = &self.inputs;
        for (input, text) in [(&inp.display_name, &a.name), (&inp.host, &a.host), (&inp.username, &a.username), (&inp.token, &String::new()), (&inp.client_id, &String::new()), (&inp.copyable, &oauth::REDIRECT_URI.into())] {
            set_input(input, text, cx);
        }
        self.stop_signin();
        self.form = Some(AccountForm {
            editing,
            provider,
            has_token: acct.is_some_and(|a| !a.token.is_empty()),
            error: None,
            advanced: false,
            provider_menu: false,
            waiting: None,
            signed_in: None,
        });
        cx.notify();
    }

    pub(super) fn set_account_provider(&mut self, provider: Provider, cx: &mut Context<Self>) {
        self.stop_signin();
        let host_input = self.inputs.host.clone();
        set_input(&self.inputs.copyable, oauth::REDIRECT_URI, cx);
        let Some(form) = &mut self.form else { return };
        form.provider = provider;
        form.provider_menu = false;
        form.signed_in = None;
        // Hosted providers have a fixed host; custom ones keep what was typed
        // unless it was a fixed host of the previous choice.
        let typed = input_text(&host_input, cx);
        let was_fixed = Provider::ALL.iter().any(|p| p.fixed_host() == Some(typed.as_str()));
        match provider.fixed_host() {
            Some(h) => set_input(&host_input, h, cx),
            None if was_fixed => set_input(&host_input, "", cx),
            None => {}
        }
        cx.notify();
    }

    pub(super) fn toggle_provider_menu(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.provider_menu = !form.provider_menu;
        }
        cx.notify();
    }

    pub(super) fn toggle_advanced(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.advanced = !form.advanced;
        }
        cx.notify();
    }

    pub(super) fn close_account_form(&mut self, cx: &mut Context<Self>) {
        self.stop_signin();
        self.form = None;
        cx.notify();
    }

    pub(super) fn save_account(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.form.clone() else { return };
        let inp = &self.inputs;
        let host = normalize_host(&input_text(&inp.host, cx));
        let username = input_text(&inp.username, cx);
        let typed_token = input_text(&inp.token, cx);
        let mut name = input_text(&inp.display_name, cx);
        if host.is_empty() {
            return self.account_form_error("Enter a host, e.g. github.com", cx);
        }
        if name.is_empty() {
            name = if username.is_empty() { host.clone() } else { format!("{username}@{host}") };
        }
        let looks_up_username = form.provider.has_oauth() && form.signed_in.is_none() && !typed_token.is_empty() && username.is_empty();
        if looks_up_username {
            let (provider, token) = (form.provider, typed_token);
            cx.spawn(async move |this, cx| {
                let res = cx.background_spawn(async move { oauth::username(provider, &host, &token) }).await;
                this.update(cx, |this, cx| match res {
                    Ok(name) => {
                        set_input(&this.inputs.username, &name, cx);
                        this.save_account(cx);
                    }
                    Err(e) => this.account_form_error(&e, cx),
                })
                .ok();
            })
            .detach();
            return;
        }
        self.stop_signin();
        let secret = form
            .signed_in
            .or_else(|| (!typed_token.is_empty()).then(|| TokenSecret { access_token: typed_token, ..TokenSecret::default() }));
        let list = &mut self.accounts;
        let existing = form.editing.as_ref().and_then(|id| list.iter_mut().find(|a| &a.id == id));
        let id = match existing {
            Some(a) => {
                a.provider = form.provider;
                (a.name, a.host, a.username) = (name, host, username);
                if let Some(s) = &secret {
                    (a.token, a.oauth) = (s.access_token.clone(), !s.client_id.is_empty());
                }
                a.id.clone()
            }
            None => {
                let id = model::new_id();
                let (token, oauth) = secret.as_ref().map_or_else(Default::default, |s| (s.access_token.clone(), !s.client_id.is_empty()));
                list.push(Account { id: id.clone(), provider: form.provider, name, host, username, oauth, token });
                id
            }
        };
        if let Some(s) = secret {
            s.store(&id);
        }
        self.persist_accounts();
        self.form = None;
        cx.notify();
    }

    pub(super) fn remove_account(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.form.as_ref().and_then(|f| f.editing.clone()) else { return };
        self.accounts.retain(|a| a.id != id);
        secrets::delete(&id);
        self.stop_signin();
        self.persist_accounts();
        self.form = None;
        cx.notify();
    }

    /// Open the browser sign-in for the form's provider; the result lands in the form.
    pub(super) fn start_signin(&mut self, cx: &mut Context<Self>) {
        self.stop_signin();
        let Some(form) = &self.form else { return };
        let provider = form.provider;
        let host = normalize_host(&input_text(&self.inputs.host, cx));
        if host.is_empty() {
            return self.account_form_error("Enter a host, e.g. github.com", cx);
        }
        let Some(app) = provider.oauth_app(&host, &input_text(&self.inputs.client_id, cx)) else {
            return self.account_form_error("Enter the OAuth client ID of your app", cx);
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = smol::channel::unbounded();
        oauth::sign_in(provider, host.clone(), app, cancel.clone(), tx);
        if let Some(form) = &mut self.form {
            (form.error, form.signed_in) = (None, None);
            form.waiting = Some(Waiting { cancel: cancel.clone(), url: None });
        }
        cx.spawn(async move |this, cx| {
            while let Ok(progress) = rx.recv().await {
                let more = this.update(cx, |this, cx| this.apply_progress(&cancel, &host, progress, cx));
                if !more.unwrap_or(false) {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    /// Apply a sign-in update to the form; `false` when it is stale or finished.
    fn apply_progress(&mut self, cancel: &Arc<AtomicBool>, host: &str, progress: Progress, cx: &mut Context<Self>) -> bool {
        let inp = self.inputs.clone();
        let Some(form) = self.form.as_mut() else { return false };
        let Some(waiting) = form.waiting.as_mut().filter(|w| Arc::ptr_eq(&w.cancel, cancel)) else { return false };
        let more = match progress {
            Progress::Open { url, user_code } => {
                if let Some(code) = user_code {
                    set_input(&inp.copyable, &code, cx);
                }
                cx.open_url(&url);
                waiting.url = Some(url);
                true
            }
            Progress::Done(res) => {
                form.waiting = None;
                match res {
                    Ok((secret, username)) => {
                        form.signed_in = Some(secret);
                        set_input(&inp.username, &username, cx);
                        if input_text(&inp.display_name, cx).is_empty() {
                            set_input(&inp.display_name, &format!("{username}@{host}"), cx);
                        }
                    }
                    Err(e) => form.error = Some(e),
                }
                false
            }
        };
        cx.notify();
        more
    }

    /// Stop waiting for the browser (the loopback listener ends; late results are ignored).
    fn stop_signin(&mut self) {
        if let Some(w) = self.form.as_mut().and_then(|f| f.waiting.take()) {
            w.cancel.store(true, Ordering::Relaxed);
        }
    }

    fn account_form_error(&mut self, msg: &str, cx: &mut Context<Self>) {
        if let Some(f) = &mut self.form {
            f.error = Some(msg.into());
        }
        cx.notify();
    }

    /// Save to disk and the secret store, and refresh the git credential registry.
    fn persist_accounts(&mut self) {
        model::save(&self.accounts);
        register_credentials(&self.accounts);
    }
}
