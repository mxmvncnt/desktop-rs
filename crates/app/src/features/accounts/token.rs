//! The secret blob of an account (access token, and for OAuth the refresh data) and
//! the blocking "give me a fresh access token" function for background threads.

use std::sync::Mutex;

use oauth2::basic::BasicClient;
use oauth2::{ClientId, RefreshToken, TokenResponse, TokenUrl};
use serde::{Deserialize, Serialize};

use super::secrets;

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TokenSecret {
    pub access_token: String,
    /// Empty = the token cannot be refreshed.
    pub refresh_token: String,
    /// Unix seconds; 0 = never expires.
    pub expires_at: i64,
    pub client_id: String,
    pub token_url: String,
}

/// Refresh this long before the token actually expires.
const REFRESH_MARGIN_SECS: i64 = 60;

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

/// HTTP client for OAuth requests (redirects must be off, see the `oauth2` docs).
pub fn http() -> ureq::Agent {
    ureq::AgentBuilder::new().redirects(0).build()
}

impl TokenSecret {
    /// From a stored string: the JSON blob, or a bare token saved by older versions.
    pub fn parse(raw: &str) -> Self {
        serde_json::from_str(raw).unwrap_or_else(|_| Self {
            access_token: raw.into(),
            ..Self::default()
        })
    }

    /// From an OAuth token response; `old_refresh` is kept when the server sends no new one.
    pub fn from_response(
        res: &impl TokenResponse,
        old_refresh: &str,
        client_id: &str,
        token_url: &str,
    ) -> Self {
        Self {
            access_token: res.access_token().secret().clone(),
            refresh_token: res
                .refresh_token()
                .map_or(old_refresh, |t| t.secret())
                .to_string(),
            expires_at: res.expires_in().map_or(0, |d| now() + d.as_secs() as i64),
            client_id: client_id.into(),
            token_url: token_url.into(),
        }
    }

    pub fn needs_refresh(&self, now: i64) -> bool {
        !self.refresh_token.is_empty()
            && self.expires_at != 0
            && now + REFRESH_MARGIN_SECS >= self.expires_at
    }

    pub fn store(&self, id: &str) {
        secrets::set(id, &serde_json::to_string(self).unwrap_or_default());
    }

    fn refresh(&self) -> Result<Self, String> {
        let url = TokenUrl::new(self.token_url.clone()).map_err(|e| e.to_string())?;
        let client = BasicClient::new(ClientId::new(self.client_id.clone())).set_token_uri(url);
        let res = client
            .exchange_refresh_token(&RefreshToken::new(self.refresh_token.clone()))
            .request(&http())
            .map_err(|e| e.to_string())?;
        Ok(Self::from_response(
            &res,
            &self.refresh_token,
            &self.client_id,
            &self.token_url,
        ))
    }
}

/// Serializes refreshes: providers rotate refresh tokens, so two at once would break one.
static REFRESH: Mutex<()> = Mutex::new(());

/// A valid access token for the account, refreshed (and saved) when it is about to expire.
/// Blocks on the secret store and the network: call it from a background thread.
pub fn fresh_token(id: &str) -> Option<String> {
    let _guard = REFRESH.lock().unwrap_or_else(|e| e.into_inner());
    let mut secret = TokenSecret::parse(&secrets::get(id)?);
    if secret.needs_refresh(now()) {
        match secret.refresh() {
            Ok(new) => {
                new.store(id);
                secret = new;
            }
            Err(e) => log::warn!("cannot refresh the token of account {id}: {e}"),
        }
    }
    Some(secret.access_token).filter(|t| !t.is_empty())
}

#[cfg(test)]
#[path = "tests/token.test.rs"]
mod tests;
