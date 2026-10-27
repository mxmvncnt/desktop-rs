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
mod ops;
mod state;
mod status_text;
#[cfg(feature = "snapshot")]
pub mod snapshot;

pub use accounts_page::AccountsPage;
pub use state::AccountsStore;
