use super::*;

#[test]
fn shortstat() {
    assert_eq!(parse_shortstat(""), LineStats::default());
    assert_eq!(parse_shortstat(" 3 files changed, 12 insertions(+), 1 deletion(-)\n"), LineStats { added: 12, deleted: 1 });
    assert_eq!(parse_shortstat(" 1 file changed, 1 insertion(+)\n"), LineStats { added: 1, deleted: 0 });
    assert_eq!(parse_shortstat(" 1 file changed, 4 deletions(-)\n"), LineStats { added: 0, deleted: 4 });
    // Binary files only: no line counts.
    assert_eq!(parse_shortstat(" 1 file changed, 0 insertions(+), 0 deletions(-)\n"), LineStats::default());
}

#[test]
fn name_status() {
    assert!(parse_name_status("").is_empty());
    let files = parse_name_status("A\0new.txt\0M\0src/a.rs\0D\0gone\0R087\0old.rs\0moved.rs\0C100\0orig\0copy\0T\0link\0");
    let got: Vec<_> = files.iter().map(|f| (f.status, f.path.as_str(), f.old_path.as_deref())).collect();
    assert_eq!(
        got,
        [
            (FileStatus::New, "new.txt", None),
            (FileStatus::Modified, "src/a.rs", None),
            (FileStatus::Deleted, "gone", None),
            (FileStatus::Renamed, "moved.rs", Some("old.rs")),
            (FileStatus::Copied, "copy", Some("orig")),
            (FileStatus::TypeChanged, "link", None),
        ]
    );
}

#[test]
fn name_status_truncated() {
    // Missing fields must not panic.
    assert_eq!(parse_name_status("R050\0old\0").len(), 1);
}
