use super::*;

#[test]
fn urls() {
    assert_eq!(
        compare_url("git@github.com:o/r.git", "main", "feat/x").unwrap(),
        "https://github.com/o/r/compare/main...feat/x?expand=1"
    );
    assert!(compare_url("https://gitlab.com/o/r.git", "main", "a b").unwrap().ends_with("target_branch%5D=main"));
    assert!(compare_url("https://example.com/o/r.git", "main", "x").is_none());
}
