//! Accounts settings state: the account list, the open add/edit form, the
//! git identity as last read, and the text inputs of both.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use gd_git::{EffectiveIdentity, Identity};
use gpui::{App, AppContext, Entity};
use gpui_elements::editable_text::{EditableTextState, StringStorage};

use super::model::{self, Account, Provider};
use super::repo_account::{self, RepoAccounts};
use super::status_text::StatusText;
use super::token::{self, TokenSecret};
use crate::repo::LoadSlot;

/// The add/edit account form (its text lives in [`AccountInputs`]).
#[derive(Clone, Debug)]
pub struct AccountForm {
    /// `None` = adding a new account.
    pub editing: Option<String>,
    pub provider: Provider,
    /// The stored account already has a token (empty token input keeps it).
    pub has_token: bool,
    pub error: Option<String>,
    /// The "use your own OAuth app" disclosure is open.
    pub advanced: bool,
    /// A browser sign-in is in progress.
    pub waiting: Option<Waiting>,
    /// The token of a finished sign-in, saved with the account.
    pub signed_in: Option<TokenSecret>,
}

/// A running browser sign-in.
#[derive(Clone, Debug)]
pub struct Waiting {
    /// Set to stop the loopback listener.
    pub cancel: Arc<AtomicBool>,
    /// Where the browser was sent, once known.
    pub url: Option<String>,
}

#[derive(Default, Clone)]
pub struct IdentityState {
    pub global: Identity,
    pub local: Identity,
    pub effective: Option<EffectiveIdentity>,
    /// Host of the open repository's remote.
    pub remote_host: Option<String>,
    pub message: Option<StatusText>,
}

#[derive(Clone)]
pub struct AccountInputs {
    pub global_name: Entity<EditableTextState>,
    pub global_email: Entity<EditableTextState>,
    pub local_name: Entity<EditableTextState>,
    pub local_email: Entity<EditableTextState>,
    pub display_name: Entity<EditableTextState>,
    pub host: Entity<EditableTextState>,
    pub username: Entity<EditableTextState>,
    pub token: Entity<EditableTextState>,
    pub client_id: Entity<EditableTextState>,
    /// Read-only text the user copies: the device code, or the redirect URI to register.
    pub copyable: Entity<EditableTextState>,
}

pub struct AccountsStore {
    pub accounts: Vec<Account>,
    /// Per repository path, the id of the account picked for it (see `account_for_repo`).
    pub repo_accounts: RepoAccounts,
    pub form: Option<AccountForm>,
    pub identity: IdentityState,
    pub inputs: AccountInputs,
    pub(super) identity_load: LoadSlot,
}

impl AccountsStore {
    /// Loads the saved accounts and registers their tokens for git network ops.
    pub fn new(cx: &mut App) -> Self {
        let mk = |cx: &mut App| cx.new(|cx| EditableTextState::new(StringStorage::default(), cx));
        let accounts = model::load();
        register_credentials(&accounts);
        gd_git::set_token_source(token::fresh_token);
        Self {
            accounts,
            repo_accounts: repo_account::load(),
            form: None,
            identity: IdentityState::default(),
            inputs: AccountInputs {
                global_name: mk(cx),
                global_email: mk(cx),
                local_name: mk(cx),
                local_email: mk(cx),
                display_name: mk(cx),
                host: mk(cx),
                username: mk(cx),
                token: mk(cx),
                client_id: mk(cx),
                copyable: mk(cx),
            },
            identity_load: LoadSlot::default(),
        }
    }
}

/// Push the accounts' tokens to the git layer.
pub fn register_credentials(accounts: &[Account]) {
    gd_git::set_credentials(accounts.iter().filter_map(Account::credential).collect());
}
