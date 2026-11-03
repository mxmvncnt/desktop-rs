use super::*;

fn oauth(expires_at: i64, refresh: &str) -> TokenSecret {
    TokenSecret {
        access_token: "a".into(),
        refresh_token: refresh.into(),
        expires_at,
        ..TokenSecret::default()
    }
}

#[test]
fn blob_round_trips_and_bare_tokens_still_parse() {
    let s = TokenSecret {
        client_id: "c".into(),
        token_url: "https://h/t".into(),
        ..oauth(100, "r")
    };
    assert_eq!(TokenSecret::parse(&serde_json::to_string(&s).unwrap()), s);
    assert_eq!(
        TokenSecret::parse("ghp_plain"),
        TokenSecret {
            access_token: "ghp_plain".into(),
            ..TokenSecret::default()
        }
    );
}

#[test]
fn refreshes_only_expiring_refreshable_tokens() {
    assert!(oauth(1000, "r").needs_refresh(950));
    assert!(oauth(1000, "r").needs_refresh(2000));
    assert!(!oauth(1000, "r").needs_refresh(900));
    assert!(!oauth(1000, "").needs_refresh(2000));
    assert!(!oauth(0, "r").needs_refresh(2000));
}
