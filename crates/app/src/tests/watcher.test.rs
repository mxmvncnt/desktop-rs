use super::*;

fn paths() -> Paths {
    Paths {
        root: "/r".into(),
        git_dir: "/r/.git".into(),
        common_dir: "/r/.git".into(),
        ignore: {
            let mut b = GitignoreBuilder::new("/r");
            b.add_line(None, "target/").unwrap();
            b.add_line(None, "node_modules/").unwrap();
            b.build().unwrap()
        },
    }
}

#[test]
fn classifies() {
    let p = paths();
    assert_eq!(classify(&p, Path::new("/r/src/a.rs")), Some(false));
    assert_eq!(classify(&p, Path::new("/r/target/debug/x")), None);
    assert_eq!(classify(&p, Path::new("/r/web/node_modules/x/y")), None);
    assert_eq!(classify(&p, Path::new("/r/.git/index")), Some(true));
    assert_eq!(classify(&p, Path::new("/r/.git/index.lock")), None);
    assert_eq!(classify(&p, Path::new("/r/.git/HEAD")), Some(true));
    assert_eq!(classify(&p, Path::new("/r/.git/refs/heads/main")), Some(true));
    assert_eq!(classify(&p, Path::new("/r/.git/refs/heads/main.lock")), None);
    assert_eq!(classify(&p, Path::new("/r/.git/objects/ab/cdef")), None);
    assert_eq!(classify(&p, Path::new("/r/.git/logs/HEAD")), None);
    assert_eq!(classify(&p, Path::new("/r/.git/FETCH_HEAD")), Some(true));
    assert_eq!(classify(&p, Path::new("/r/.git/rebase-merge")), Some(true));
}
