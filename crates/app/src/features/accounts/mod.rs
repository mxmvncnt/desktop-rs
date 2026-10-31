//! Accounts: the global git identity and the hosting-provider accounts whose tokens
//! authenticate HTTPS network ops, shown as the "Accounts" page of the settings modal. The
//! per-repository identity override and account pick (`RepoIdentity`, `RepoAccountBlock`) are
//! pages of the repository settings modal. Git side:
//! `gd-git/src/identity.rs` and `credentials.rs`.

mod account_form;
mod account_list;
mod account_row;
mod accounts_page;
mod identity_block;
mod identity_section;
mod model;
mod oauth;
mod ops;
mod repo_account;
mod repo_account_block;
mod repo_identity;
mod secrets;
mod state;
mod status_text;
mod token;
#[cfg(feature = "snapshot")]
pub mod snapshot;

pub use accounts_page::AccountsPage;
pub use repo_account_block::RepoAccountBlock;
pub use repo_identity::RepoIdentity;
#[allow(unused_imports)] // for the CI page
pub use repo_account::account_for_repo;
pub use state::AccountsStore;
#[allow(unused_imports)] // for the CI page
pub use token::fresh_token;
