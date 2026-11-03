//! Provider accounts and their persistence.
//!
//! Metadata (everything but the token) is in `<config dir>/accounts.toml`;
//! tokens are in the secret store (`secrets.rs`) as a `TokenSecret`, keyed by account id.

use gd_git::HostCredential;
use serde::{Deserialize, Serialize};

use super::oauth::{GITEA_REDIRECT_URI, REDIRECT_URI};
use super::secrets;
use super::token::TokenSecret;
use crate::features::settings::config_dir;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    GitHub,
    GitHubEnterprise,
    GitLab,
    Gitea,
    Forgejo,
    Bitbucket,
    Generic,
}

impl Provider {
    pub const ALL: [Provider; 7] = [
        Self::GitHub,
        Self::GitHubEnterprise,
        Self::GitLab,
        Self::Gitea,
        Self::Forgejo,
        Self::Bitbucket,
        Self::Generic,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::GitHubEnterprise => "Enterprise",
            Self::GitLab => "GitLab",
            Self::Gitea => "Gitea",
            Self::Forgejo => "Forgejo",
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
            Self::GitHubEnterprise | Self::Gitea | Self::Forgejo | Self::Generic => None,
        }
    }

    /// Signs in with the browser (OAuth) instead of a typed token.
    pub fn has_oauth(self) -> bool {
        !matches!(self, Self::Bitbucket | Self::Generic)
    }

    /// GitHub uses the device flow; the others the authorization code flow with PKCE.
    pub fn uses_device_flow(self) -> bool {
        matches!(self, Self::GitHub | Self::GitHubEnterprise)
    }
    /// The OAuth app to sign in with on `host`: the user's own (`custom` client id) wins over the built-in
    /// one of the host, then the app Gitea and Forgejo preregister on every instance. `None` = no app
    /// is available, the user pastes a token instead.
    pub fn oauth_app(self, host: &str, custom: &str) -> Option<OAuthApp> {
        let custom = custom.trim();
        if !custom.is_empty() {
            return Some(OAuthApp {
                client_id: custom.into(),
                redirect_uri: REDIRECT_URI,
            });
        }
        let gitea = matches!(self, Self::Gitea | Self::Forgejo);
        let builtin = BUILTIN_CLIENT_IDS
            .iter()
            .find(|(h, id)| *h == host && !id.is_empty())
            .map(|(_, id)| *id);
        let client_id = builtin.or(gitea.then_some(GITEA_CLIENT_ID))?;
        Some(OAuthApp {
            client_id: client_id.into(),
            redirect_uri: if gitea {
                GITEA_REDIRECT_URI
            } else {
                REDIRECT_URI
            },
        })
    }
}

/// How the add/edit form gets the token.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SignInMethod {
    Browser,
    Token,
    OwnApp,
}

impl SignInMethod {
    /// The browser when the host has an OAuth app, else a pasted token.
    pub fn default_for(provider: Provider, host: &str) -> Self {
        if provider.oauth_app(host, "").is_some() {
            Self::Browser
        } else {
            Self::Token
        }
    }
}

/// An OAuth app to sign in with and the redirect URI of the loopback listener it accepts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OAuthApp {
    pub client_id: String,
    pub redirect_uri: &'static str,
}

/// Client ids are public (device flow and PKCE need no secret). An empty id = not registered yet.
const BUILTIN_CLIENT_IDS: [(&str, &str); 4] = [
    // OAuth App "desktop-rs" (device flow enabled) of github.com/mxmvncnt.
    ("github.com", "Ov23liWNVoyi5E8E5lb0"),
    // Application "desktop-rs" of gitlab.com (confidential unchecked, scope `api`, redirect URI `http://127.0.0.1:47123/callback`).
    (
        "gitlab.com",
        "4f94f81b48f3ee98ae4959ab9a411bca5c4a747caa2bbb8404317c3cdc122929",
    ),
    // Optional (until then the Gitea/Forgejo default app below is used): register an OAuth2 app on the
    // host, confidential unchecked, redirect URI `http://127.0.0.1:47123` (no path).
    ("codeberg.org", ""),
    ("gitea.com", ""),
];

/// "git-credential-oauth", preregistered public app of Gitea and Forgejo (`DEFAULT_APPLICATIONS`);
/// its redirect URI is `http://127.0.0.1`, which accepts any loopback port but no path.
const GITEA_CLIENT_ID: &str = "a4792ccc-144e-407e-86c9-5e7d8d9c3269";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub provider: Provider,
    pub name: String,
    pub host: String,
    pub username: String,
    /// Signed in with OAuth: the token expires and is refreshed (`token::fresh_token`).
    #[serde(default)]
    pub oauth: bool,
    /// The access token as of loading, for the "token saved" note and as the credential's
    /// fallback; lives in the secret store, never in the file.
    #[serde(skip)]
    pub token: String,
}

impl Account {
    /// Credential for git network ops (`None` without a token or host).
    pub fn credential(&self) -> Option<HostCredential> {
        (!self.token.is_empty() && !self.host.is_empty()).then(|| HostCredential {
            id: if self.oauth {
                self.id.clone()
            } else {
                String::new()
            },
            host: self.host.clone(),
            // GitLab wants `oauth2` as the user of OAuth tokens (any user works for its personal tokens).
            username: match (self.provider, self.username.as_str()) {
                (Provider::GitLab, _) => "oauth2".into(),
                (_, "") => "git".into(),
                (_, user) => user.into(),
            },
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

pub fn load() -> Vec<Account> {
    let Some(path) = config_dir().map(|d| d.join("accounts.toml")) else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let mut file: AccountsFile = toml::from_str(&text).unwrap_or_else(|e| {
        log::error!("cannot parse {}: {e}", path.display());
        AccountsFile::default()
    });
    for a in &mut file.accounts {
        a.token = secrets::get(&a.id)
            .map(|raw| TokenSecret::parse(&raw).access_token)
            .unwrap_or_default();
    }
    file.accounts
}

/// Write the metadata; secrets are stored when an account is saved or signed in.
pub fn save(list: &[Account]) {
    let Some(dir) = config_dir() else { return };
    let text = toml::to_string(&AccountsFile {
        accounts: list.to_vec(),
    })
    .unwrap_or_default();
    if let Err(e) =
        std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(dir.join("accounts.toml"), text))
    {
        log::error!("cannot save accounts: {e}");
    }
}

#[cfg(test)]
#[path = "tests/model.test.rs"]
mod tests;
