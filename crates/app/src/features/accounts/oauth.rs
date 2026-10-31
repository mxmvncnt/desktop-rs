//! "Sign in with browser": the OAuth device flow (GitHub) or the authorization code flow with
//! PKCE and a loopback redirect (GitLab, Gitea, Forgejo). Blocking, so it runs on its own thread
//! and reports through a channel. The only API call is `/user`, to learn the username.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, DeviceAuthorizationUrl, PkceCodeChallenge, RedirectUrl, Scope,
    StandardDeviceAuthorizationResponse, TokenUrl,
};
use smol::channel::Sender;

use super::model::Provider;
use super::token::{TokenSecret, http};

/// Fixed so the user can register it once in their OAuth app.
pub const REDIRECT_URI: &str = "http://127.0.0.1:47123/callback";

/// How long the loopback listener waits for the browser.
const CALLBACK_WAIT_SECS: u32 = 300;

pub enum Progress {
    /// Open `url` in the browser; the device flow also shows `user_code`.
    Open { url: String, user_code: Option<String> },
    Done(Result<(TokenSecret, String), String>),
}

fn scopes(provider: Provider) -> &'static [&'static str] {
    match provider {
        Provider::GitHub | Provider::GitHubEnterprise => &["repo", "workflow", "read:user"],
        Provider::GitLab => &["api"],
        _ => &["read:user", "write:repository"],
    }
}

/// Run the sign-in of `provider` on `host` off-thread. `cancel` stops the loopback listener
/// (the device flow just keeps polling until its code expires; its result is ignored).
pub fn sign_in(provider: Provider, host: String, client_id: String, cancel: Arc<AtomicBool>, tx: Sender<Progress>) {
    std::thread::spawn(move || {
        let base = format!("https://{host}");
        let (auth, token) = match provider {
            Provider::GitLab => ("oauth/authorize", "oauth/token"),
            _ => ("login/oauth/authorize", "login/oauth/access_token"),
        };
        let (auth_url, token_url) = (format!("{base}/{auth}"), format!("{base}/{token}"));
        let res = if provider.uses_device_flow() {
            device_flow(provider, &client_id, &auth_url, &token_url, &format!("{base}/login/device/code"), &tx)
        } else {
            code_flow(provider, &client_id, &auth_url, &token_url, &cancel, &tx)
        };
        let res = res.and_then(|secret| {
            let name = username(provider, &host, &secret.access_token)?;
            Ok((secret, name))
        });
        tx.send_blocking(Progress::Done(res)).ok();
    });
}

fn device_flow(
    provider: Provider,
    client_id: &str,
    auth_url: &str,
    token_url: &str,
    device_url: &str,
    tx: &Sender<Progress>,
) -> Result<TokenSecret, String> {
    let err = |e: &dyn std::fmt::Display| e.to_string();
    let client = BasicClient::new(ClientId::new(client_id.into()))
        .set_auth_uri(AuthUrl::new(auth_url.into()).map_err(|e| err(&e))?)
        .set_token_uri(TokenUrl::new(token_url.into()).map_err(|e| err(&e))?)
        .set_device_authorization_url(DeviceAuthorizationUrl::new(device_url.into()).map_err(|e| err(&e))?);
    let http = http();
    let details: StandardDeviceAuthorizationResponse = client
        .exchange_device_code()
        .add_scopes(scopes(provider).iter().map(|s| Scope::new(s.to_string())))
        .request(&http)
        .map_err(|e| err(&e))?;
    tx.send_blocking(Progress::Open {
        url: details.verification_uri().to_string(),
        user_code: Some(details.user_code().secret().clone()),
    })
    .ok();
    let res = client
        .exchange_device_access_token(&details)
        .request(&http, std::thread::sleep, None)
        .map_err(|e| err(&e))?;
    Ok(TokenSecret::from_response(&res, "", client_id, token_url))
}

fn code_flow(
    provider: Provider,
    client_id: &str,
    auth_url: &str,
    token_url: &str,
    cancel: &AtomicBool,
    tx: &Sender<Progress>,
) -> Result<TokenSecret, String> {
    let err = |e: &dyn std::fmt::Display| e.to_string();
    let client = BasicClient::new(ClientId::new(client_id.into()))
        .set_auth_uri(AuthUrl::new(auth_url.into()).map_err(|e| err(&e))?)
        .set_token_uri(TokenUrl::new(token_url.into()).map_err(|e| err(&e))?)
        .set_redirect_uri(RedirectUrl::new(REDIRECT_URI.into()).map_err(|e| err(&e))?);
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let (url, state) = client
        .authorize_url(CsrfToken::new_random)
        .set_pkce_challenge(challenge)
        .add_scopes(scopes(provider).iter().map(|s| Scope::new(s.to_string())))
        .url();
    let server = tiny_http::Server::http("127.0.0.1:47123").map_err(|e| format!("cannot listen on 127.0.0.1:47123: {e}"))?;
    tx.send_blocking(Progress::Open { url: url.to_string(), user_code: None }).ok();

    let page = tiny_http::Header::from_bytes("Content-Type", "text/html; charset=utf-8").map_err(|()| "bad header".to_string())?;
    let mut code = None;
    for _ in 0..CALLBACK_WAIT_SECS {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        let Some(request) = server.recv_timeout(Duration::from_secs(1)).map_err(|e| err(&e))? else { continue };
        let callback = url::Url::parse(REDIRECT_URI).and_then(|base| base.join(request.url())).map_err(|e| err(&e))?;
        let ours = callback.path() == "/callback";
        let query = |key: &str| callback.query_pairs().find(|(k, _)| k == key).map(|(_, v)| v.into_owned());
        let body = if ours { "You can close this tab and return to desktop-rs." } else { "Not found" };
        request.respond(tiny_http::Response::from_string(body).with_header(page.clone())).ok();
        if !ours {
            continue;
        }
        if let Some(e) = query("error") {
            return Err(query("error_description").unwrap_or(e));
        }
        if query("state").as_deref() != Some(state.secret()) {
            return Err("the sign-in response did not match the request (state)".into());
        }
        code = query("code");
        break;
    }
    let code = code.ok_or("timed out waiting for the browser")?;
    let res = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(verifier)
        .request(&http())
        .map_err(|e| err(&e))?;
    Ok(TokenSecret::from_response(&res, "", client_id, token_url))
}

/// The login of the token's owner (`/user` of the provider's API).
fn username(provider: Provider, host: &str, token: &str) -> Result<String, String> {
    let (url, field) = match provider {
        Provider::GitHub => ("https://api.github.com/user".to_string(), "login"),
        Provider::GitHubEnterprise => (format!("https://{host}/api/v3/user"), "login"),
        Provider::GitLab => (format!("https://{host}/api/v4/user"), "username"),
        _ => (format!("https://{host}/api/v1/user"), "login"),
    };
    let user: serde_json::Value = http()
        .get(&url)
        .set("Authorization", &format!("Bearer {token}"))
        .set("User-Agent", "desktop-rs")
        .call()
        .map_err(|e| format!("cannot read the user from {url}: {e}"))?
        .into_json()
        .map_err(|e| e.to_string())?;
    user[field].as_str().map(str::to_string).ok_or_else(|| format!("no `{field}` in the response of {url}"))
}
