use super::*;

fn acct(id: &str, provider: Provider, token: &str) -> Account {
    Account { id: id.into(), provider, name: "Work".into(), host: "github.com".into(), username: "me".into(), oauth: false, token: token.into() }
}

#[test]
fn metadata_round_trips_without_tokens() {
    let list = vec![acct("a1", Provider::GitHubEnterprise, "ghp_x"), acct("b2", Provider::Generic, "")];
    let text = toml::to_string(&AccountsFile { accounts: list.clone() }).unwrap();
    assert!(!text.contains("ghp_x"));
    let back = toml::from_str::<AccountsFile>(&text).unwrap().accounts;
    assert_eq!(back, list.iter().map(|a| Account { token: String::new(), ..a.clone() }).collect::<Vec<_>>());
}

#[test]
fn normalizes_hosts() {
    assert_eq!(normalize_host(" https://User@Git.Corp:8443/x/y "), "git.corp:8443");
    assert_eq!(normalize_host("github.com"), "github.com");
}

#[test]
fn credential_needs_token_and_defaults_username() {
    let mut a = acct("a", Provider::GitHub, "");
    assert!(a.credential().is_none());
    a.token = "t".into();
    a.username.clear();
    assert_eq!(a.credential().unwrap().username, "git");
    a.provider = Provider::GitLab;
    assert_eq!(a.credential().unwrap().username, "oauth2");
}

#[test]
fn oauth_app_resolution_order() {
    let id = |p: Provider, host, custom| p.oauth_app(host, custom).map(|a| a.client_id);
    // custom > built-in > Gitea default > none (paste a token)
    assert_eq!(id(Provider::Forgejo, "codeberg.org", " mine "), Some("mine".into()));
    assert_eq!(id(Provider::Forgejo, "codeberg.org", ""), Some(GITEA_CLIENT_ID.into()));
    assert_eq!(id(Provider::Gitea, "git.example.com", ""), Some(GITEA_CLIENT_ID.into()));
    assert_eq!(id(Provider::GitHubEnterprise, "ghe.example.com", ""), None);
    assert_eq!(id(Provider::GitLab, "gitlab.example.com", "x"), Some("x".into()));
    // an empty built-in id (not registered yet) counts as none
    assert_eq!(id(Provider::GitHub, "github.com", ""), None);
}

#[test]
fn redirect_uri_depends_on_the_app() {
    let uri = |p: Provider, custom| p.oauth_app("git.example.com", custom).unwrap().redirect_uri;
    assert_eq!(uri(Provider::Gitea, ""), GITEA_REDIRECT_URI);
    assert_eq!(uri(Provider::Gitea, "mine"), REDIRECT_URI);
    assert_eq!(uri(Provider::GitLab, "mine"), REDIRECT_URI);
}

#[test]
fn sign_in_method_defaults_to_the_browser_when_an_app_exists() {
    assert_eq!(SignInMethod::default_for(Provider::Gitea, "git.example.com"), SignInMethod::Browser);
    assert_eq!(SignInMethod::default_for(Provider::GitHubEnterprise, "ghe.example.com"), SignInMethod::Token);
    assert_eq!(SignInMethod::default_for(Provider::Bitbucket, "bitbucket.org"), SignInMethod::Token);
    // github.com has no registered app yet
    assert_eq!(SignInMethod::default_for(Provider::GitHub, "github.com"), SignInMethod::Token);
}
