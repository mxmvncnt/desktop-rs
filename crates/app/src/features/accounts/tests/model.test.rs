use super::*;

fn acct(id: &str, provider: Provider, token: &str) -> Account {
    Account { id: id.into(), provider, name: "Work".into(), host: "github.com".into(), username: "me".into(), token: token.into() }
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
}
