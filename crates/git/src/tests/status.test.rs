use super::{FileStatus, parse_status};

#[test]
fn headers_and_entries() {
    let out = b"# branch.oid abc\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0\
1 .M N... 100644 100644 100644 h h src/b.rs\0\
2 R. N... 100644 100644 100644 h h R100 new name.rs\0old.rs\0\
u UU N... 1 2 3 4 h h h conflict.rs\0\
? a.txt\0";
    let st = parse_status(out).unwrap();
    assert_eq!(st.branch.as_deref(), Some("main"));
    assert_eq!((st.ahead_behind.ahead, st.ahead_behind.behind), (2, 1));
    let files: Vec<_> = st.files.iter().map(|f| (f.path.as_str(), f.status)).collect();
    assert_eq!(
        files,
        [
            ("conflict.rs", FileStatus::Conflicted),
            ("a.txt", FileStatus::Untracked),
            ("new name.rs", FileStatus::Renamed),
            ("src/b.rs", FileStatus::Modified),
        ]
    );
    assert_eq!(st.files[2].old_path.as_deref(), Some("old.rs"));
}

#[test]
fn initial_and_detached() {
    let st = parse_status(b"# branch.oid (initial)\0# branch.head (detached)\0").unwrap();
    assert!(st.head_oid.is_none() && st.branch.is_none());
}

#[test]
fn malformed_output_is_an_error_not_a_panic() {
    for bad in [&b"1 M\0"[..], b"2 R. x\0", b"u UU\0", b"# branch.ab +x\0", b"# branch.ab\0", b"?\0", b"1\0"] {
        assert!(parse_status(bad).is_err(), "{}", String::from_utf8_lossy(bad));
    }
}
