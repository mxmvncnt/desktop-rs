//! Which account and repository the CI page talks to, and why it can't.

use std::path::PathBuf;

use gd_forge::{Client, Error, Forge, RepoRef};
use gd_git::Repository;

use crate::features::accounts::{Account, Provider, RepoAccounts, account_for_repo, fresh_token};

/// The forge account serving the open repository.
#[derive(Clone, Debug)]
pub struct Target {
    pub forge: Forge,
    pub repo: RepoRef,
    /// Working tree, where workflow files are read from.
    pub root: PathBuf,
    pub branch: Option<String>,
    pub account_id: String,
    pub account_name: String,
    pub host: String,
}

/// Why there is nothing to show; each variant has its own empty state.
#[derive(Clone, Debug, PartialEq)]
pub enum Problem {
    NoRemote,
    /// Host of the remote.
    NoAccount(String),
    /// Provider of the account (Bitbucket, Generic).
    Unsupported(&'static str),
    Unauthorized,
    RateLimited(Option<u64>),
    NotFound,
    Failed(String),
}

impl From<Error> for Problem {
    fn from(e: Error) -> Self {
        match e {
            Error::Unauthorized => Self::Unauthorized,
            Error::RateLimited(s) => Self::RateLimited(s),
            Error::NotFound => Self::NotFound,
            e => Self::Failed(e.to_string()),
        }
    }
}

impl Problem {
    /// Title and explanation for the empty state.
    pub fn describe(&self) -> (String, String) {
        let (title, body) = match self {
            Self::NoRemote => ("No remote".to_string(), "This repository has no remote, so there are no pipelines to show.".to_string()),
            Self::NoAccount(host) => (format!("No account for {host}"), format!("Add an account for {host} in Settings → Accounts.")),
            Self::Unsupported(label) => (
                "CI is not supported for this account type".into(),
                format!("{label} accounts can't list pipelines. Pick another account in the repository settings."),
            ),
            Self::Unauthorized => ("Sign in again".into(), "The token was rejected or has expired.".into()),
            Self::RateLimited(wait) => {
                ("Rate limited".into(), format!("The server asked to slow down{}.", wait.map(|s| format!(", try again in {s}s")).unwrap_or_default()))
            }
            Self::NotFound => {
                ("No pipelines found".into(), "CI is not enabled for this repository, or the account has no access to it.".into())
            }
            Self::Failed(message) => ("Couldn't load pipelines".into(), message.clone()),
        };
        (title, body)
    }
}

impl Target {
    /// A client with a fresh token. Blocks (secret store, maybe the network): background threads only.
    pub fn client(&self) -> Result<Client, Error> {
        let token = fresh_token(&self.account_id).ok_or(Error::Unauthorized)?;
        Ok(Client::new(self.forge, &self.host, &token))
    }

    /// Blocks on git: background threads only.
    pub fn resolve(repo: &Repository, branch: Option<String>, accounts: &[Account], picks: &RepoAccounts) -> Result<Self, Problem> {
        let remote = repo.tracking_remote_url().and_then(|url| RepoRef::from_remote(&url)).ok_or(Problem::NoRemote)?;
        let account = account_for_repo(accounts, picks, repo.root(), &remote.host).ok_or_else(|| Problem::NoAccount(remote.host.clone()))?;
        let forge = forge_of(account.provider).ok_or(Problem::Unsupported(account.provider.full_label()))?;
        Ok(Self {
            forge,
            host: remote.host.clone(),
            repo: remote,
            root: repo.root().to_path_buf(),
            branch,
            account_id: account.id.clone(),
            account_name: account.name.clone(),
        })
    }
}

pub fn forge_of(provider: Provider) -> Option<Forge> {
    match provider {
        Provider::GitHub | Provider::GitHubEnterprise => Some(Forge::GitHub),
        Provider::GitLab => Some(Forge::GitLab),
        Provider::Gitea => Some(Forge::Gitea),
        Provider::Forgejo => Some(Forge::Forgejo),
        Provider::Bitbucket | Provider::Generic => None,
    }
}

#[cfg(test)]
#[path = "tests/target.test.rs"]
mod tests;
