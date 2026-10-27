//! Provider accounts and their persistence.
//!
//! Metadata (everything but the token) is in `<config dir>/accounts.toml`;
//! tokens are in the OS keyring (service `desktop-rs`, user = account id).

use gd_git::HostCredential;
use serde::{Deserialize, Serialize};

use crate::features::settings::config_dir;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    GitHub,
    GitHubEnterprise,
    GitLab,
    Bitbucket,
    Generic,
}

impl Provider {
    pub const ALL: [Provider; 5] =
        [Self::GitHub, Self::GitHubEnterprise, Self::GitLab, Self::Bitbucket, Self::Generic];

    pub fn label(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::GitHubEnterprise => "Enterprise",
            Self::GitLab => "GitLab",
            Self::Bitbucket => "Bitbucket",
            Self::Generic => "Generic",
        }
    }

    pub fn full_label(self) -> &'static str {
        match self {
            Self::GitHubEnterprise => "GitHub Enterprise",
            p => p.label(),
        }
    }

    /// Fixed host of hosted providers; `None` = the user types it.
    pub fn fixed_host(self) -> Option<&'static str> {
        match self {
            Self::GitHub => Some("github.com"),
            Self::GitLab => Some("gitlab.com"),
            Self::Bitbucket => Some("bitbucket.org"),
            Self::GitHubEnterprise | Self::Generic => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub provider: Provider,
    pub name: String,
    pub host: String,
    pub username: String,
    /// Kept in memory for the credential helper; lives in the keyring, never in the file.
    #[serde(skip)]
    pub token: String,
}

impl Account {
    /// Credential for git network ops (`None` without a token or host).
    pub fn credential(&self) -> Option<HostCredential> {
        (!self.token.is_empty() && !self.host.is_empty()).then(|| HostCredential {
            host: self.host.clone(),
            username: if self.username.is_empty() { "git".into() } else { self.username.clone() },
            token: self.token.clone(),
        })
    }
}

/// `https://Git.Corp/path/` -> `git.corp`; keeps an explicit port.
pub fn normalize_host(input: &str) -> String {
    let s = input.trim();
    let s = s.split_once("://").map_or(s, |(_, rest)| rest);
    let s = s.split('/').next().unwrap_or("");
    let s = s.rsplit_once('@').map_or(s, |(_, h)| h);
    s.trim().to_lowercase()
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Default, Serialize, Deserialize)]
struct AccountsFile {
    accounts: Vec<Account>,
}

const KEYRING_SERVICE: &str = "desktop-rs";

pub fn load() -> Vec<Account> {
    let Some(path) = config_dir().map(|d| d.join("accounts.toml")) else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(&path) else { return Vec::new() };
    let mut file: AccountsFile = toml::from_str(&text).unwrap_or_else(|e| {
        log::error!("cannot parse {}: {e}", path.display());
        AccountsFile::default()
    });
    for a in &mut file.accounts {
        a.token = keyring::Entry::new(KEYRING_SERVICE, &a.id).and_then(|e| e.get_password()).unwrap_or_default();
    }
    file.accounts
}

pub fn save(list: &[Account]) {
    let Some(dir) = config_dir() else { return };
    let text = toml::to_string(&AccountsFile { accounts: list.to_vec() }).unwrap_or_default();
    if let Err(e) = std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(dir.join("accounts.toml"), text)) {
        log::error!("cannot save accounts: {e}");
    }
    for a in list {
        if let Err(e) = store_token(&a.id, &a.token) {
            log::error!("cannot store token in keyring: {e}");
        }
    }
}

/// Store the token in the keyring; an empty token removes the entry.
pub fn store_token(id: &str, token: &str) -> keyring::Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, id)?;
    if !token.is_empty() {
        return entry.set_password(token);
    }
    match entry.delete_credential() {
        Err(keyring::Error::NoEntry) => Ok(()),
        r => r,
    }
}

#[cfg(test)]
#[path = "tests/model.test.rs"]
mod tests;
