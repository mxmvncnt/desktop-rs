use super::*;

#[test]
fn providers_map_to_forges() {
    assert_eq!(forge_of(Provider::GitHubEnterprise), Some(Forge::GitHub));
    assert_eq!(forge_of(Provider::Forgejo), Some(Forge::Forgejo));
    assert_eq!(forge_of(Provider::Bitbucket), None);
    assert_eq!(forge_of(Provider::Generic), None);
}

#[test]
fn api_errors_become_problems() {
    assert_eq!(Problem::from(Error::Unauthorized), Problem::Unauthorized);
    assert_eq!(Problem::from(Error::RateLimited(Some(5))), Problem::RateLimited(Some(5)));
    assert_eq!(Problem::from(Error::NotFound), Problem::NotFound);
    assert!(matches!(Problem::from(Error::Forbidden), Problem::Failed(_)));
}
