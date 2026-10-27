//! `impl AccountsStore`: git identity load/save (background `git config`) and
//! account add/edit/remove.

use gd_git::{Identity, Repository};
use gpui::{AppContext as _, Context, Entity};
use gpui_elements::editable_text::EditableTextState;

use super::model::{self, Account, Provider, normalize_host};
use super::state::{AccountForm, AccountsStore, register_credentials};
use super::status_text::StatusText;
use crate::repo::spawn_load;

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
                    Some(r) => Some((r.local_identity()?, r.effective_identity()?)),
                    None => None,
                };
                Ok::<_, gd_git::GitError>((global, repo_part))
            },
            |this, res, cx| this.apply_identity(res, cx),
            cx,
        );
    }

    fn apply_identity(
        &mut self,
        res: Result<(Identity, Option<(Identity, gd_git::EffectiveIdentity)>), gd_git::GitError>,
        cx: &mut Context<Self>,
    ) {
        match res {
            Ok((global, repo_part)) => {
                let id = &mut self.identity;
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

    pub(super) fn save_global_identity(&mut self, open: Option<Repository>, cx: &mut Context<Self>) {
        let inp = &self.inputs;
        let id = Identity { name: input_text(&inp.global_name, cx), email: input_text(&inp.global_email, cx) };
        self.write_identity(None, open, id, "Saved to your global git config", cx);
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
            token: String::new(),
        });
        let inp = &self.inputs;
        for (input, text) in [(&inp.display_name, &a.name), (&inp.host, &a.host), (&inp.username, &a.username), (&inp.token, &String::new())] {
            set_input(input, text, cx);
        }
        self.form =
            Some(AccountForm { editing, provider, has_token: acct.is_some_and(|a| !a.token.is_empty()), error: None });
        cx.notify();
    }

    pub(super) fn set_account_provider(&mut self, provider: Provider, cx: &mut Context<Self>) {
        let host_input = self.inputs.host.clone();
        let Some(form) = &mut self.form else { return };
        form.provider = provider;
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

    pub(super) fn close_account_form(&mut self, cx: &mut Context<Self>) {
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
        let list = &mut self.accounts;
        let existing = form.editing.as_ref().and_then(|id| list.iter_mut().find(|a| &a.id == id));
        match existing {
            Some(a) => {
                a.provider = form.provider;
                (a.name, a.host, a.username) = (name, host, username);
                if !typed_token.is_empty() {
                    a.token = typed_token;
                }
            }
            None => list.push(Account { id: model::new_id(), provider: form.provider, name, host, username, token: typed_token }),
        }
        self.persist_accounts();
        self.form = None;
        cx.notify();
    }

    pub(super) fn remove_account(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.form.as_ref().and_then(|f| f.editing.clone()) else { return };
        self.accounts.retain(|a| a.id != id);
        if let Err(e) = model::store_token(&id, "") {
            log::error!("cannot remove token from keyring: {e}");
        }
        self.persist_accounts();
        self.form = None;
        cx.notify();
    }

    fn account_form_error(&mut self, msg: &str, cx: &mut Context<Self>) {
        if let Some(f) = &mut self.form {
            f.error = Some(msg.into());
        }
        cx.notify();
    }

    /// Save to disk and the keyring, and refresh the git credential registry.
    fn persist_accounts(&mut self) {
        model::save(&self.accounts);
        register_credentials(&self.accounts);
    }
}
