mod common;

use gd_git::{Identity, IdentitySource, Repository};

// Global config is process-wide, so this file holds a single test.
#[test]
fn identity() {
    let tmp = common::tempdir();
    let dir = tmp.path().to_path_buf();
    // Never touch the real global config: libgit2 reads `<dir>/.gitconfig`, the git CLI writes `$GIT_CONFIG_GLOBAL`.
    // SAFETY: single-threaded at this point.
    unsafe {
        std::env::set_var("GIT_CONFIG_GLOBAL", dir.join(".gitconfig"));
        for level in [git2::ConfigLevel::Global, git2::ConfigLevel::XDG, git2::ConfigLevel::System] {
            git2::opts::set_search_path(level, &dir).unwrap();
        }
    }
    let repo = Repository::init(dir.join("repo")).unwrap();

    assert_eq!(gd_git::global_identity().unwrap(), Identity::default());
    assert_eq!(repo.effective_identity().unwrap().source, IdentitySource::Unset);

    let g = Identity { name: "Global Gal".into(), email: "gal@example.com".into() };
    gd_git::set_global_identity(&g).unwrap();
    assert_eq!(gd_git::global_identity().unwrap(), g);
    let eff = repo.effective_identity().unwrap();
    assert_eq!((eff.identity.clone(), eff.source), (g.clone(), IdentitySource::Global));

    let l = Identity { name: "Local Lou".into(), email: String::new() };
    repo.set_local_identity(&l).unwrap();
    let eff = repo.effective_identity().unwrap();
    assert_eq!(eff.source, IdentitySource::Repository);
    assert_eq!((eff.identity.name.as_str(), eff.identity.email.as_str()), ("Local Lou", "gal@example.com"));

    repo.set_local_identity(&Identity::default()).unwrap();
    assert_eq!(repo.effective_identity().unwrap().source, IdentitySource::Global);
    gd_git::set_global_identity(&Identity::default()).unwrap();
    assert_eq!(gd_git::global_identity().unwrap(), Identity::default());
}
