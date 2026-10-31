//! Accounts: the git identity (global + per-repository override) and the
//! hosting-provider accounts whose tokens authenticate HTTPS network ops.
//! Shown as the "Accounts" page of the settings modal. Git side:
//! `gd-git/src/identity.rs` and `credentials.rs`.

mod account_form;
mod account_list;
mod account_row;
mod accounts_page;
mod field_row;
mod identity_block;
mod identity_section;
mod model;
mod oauth;
mod ops;
mod repo_account;
mod repo_account_block;
mod secrets;
mod state;
mod status_text;
mod token;
#[cfg(feature = "snapshot")]
pub mod snapshot;

pub use accounts_page::AccountsPage;
#[allow(unused_imports)] // for the CI page
pub use repo_account::account_for_repo;
pub use state::AccountsStore;
#[allow(unused_imports)] // for the CI page
pub use token::fresh_token;
