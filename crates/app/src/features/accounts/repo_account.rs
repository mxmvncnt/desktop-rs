//! Which account serves a repository (CI, API calls): the one the user picked for it, else the
//! first whose host is the repository's remote host. Picks are in `<config dir>/repo_accounts.toml`
//! (repository path -> account id), not in the repository.

use std::collections::BTreeMap;
use std::path::Path;

use super::model::Account;
use crate::features::settings::config_dir;

pub type RepoAccounts = BTreeMap<String, String>;

/// The account for the repository at `repo_path` whose remote is on `remote_host`.
pub fn account_for_repo<'a>(
    accounts: &'a [Account],
    picks: &RepoAccounts,
    repo_path: &Path,
    remote_host: &str,
) -> Option<&'a Account> {
    let picked = picks.get(repo_path.to_string_lossy().as_ref()).and_then(|id| accounts.iter().find(|a| &a.id == id));
    let host = remote_host.to_lowercase();
    picked.or_else(|| accounts.iter().find(|a| a.host.split(':').next() == Some(host.as_str())))
}

pub fn load() -> RepoAccounts {
    let text = config_dir().and_then(|d| std::fs::read_to_string(d.join("repo_accounts.toml")).ok());
    text.and_then(|t| toml::from_str(&t).ok()).unwrap_or_default()
}

pub fn save(picks: &RepoAccounts) {
    let Some(dir) = config_dir() else { return };
    let text = toml::to_string(picks).unwrap_or_default();
    if let Err(e) = std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(dir.join("repo_accounts.toml"), text)) {
        log::error!("cannot save repo_accounts.toml: {e}");
    }
}

#[cfg(test)]
#[path = "tests/repo_account.test.rs"]
mod tests;
