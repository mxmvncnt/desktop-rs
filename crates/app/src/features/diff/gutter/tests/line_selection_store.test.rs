//! Deselected lines must stay deselected when the file changes on disk, and a
//! commit must stage what the gutter shows.
//! Needs gpui's test platform, hence the `snapshot` feature (`cargo test --features snapshot`).

use gpui::{AppContext, TestAppContext};

use super::*;

#[gpui::test]
fn selection_follows_edits(cx: &mut TestAppContext) {
    // Not a tempdir: deleting the repo at the end would wake the file watcher's thread.
    let repo = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/test-scratch/line-selection"
    ));
    std::fs::remove_dir_all(repo).ok();
    std::fs::create_dir_all(repo).unwrap();
    let git = |args: &[&str]| {
        assert!(
            std::process::Command::new("git")
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(args)
                .current_dir(repo)
                .status()
                .unwrap()
                .success()
        )
    };
    let write = |name: &str, text: &str| std::fs::write(repo.join(name), text).unwrap();
    git(&["init", "-q"]);
    write("a.txt", "1\n2\n3\n");
    write("b.txt", "1\n2\n3\n");
    git(&["add", "."]);
    git(&["commit", "-qm", "init"]);
    write("a.txt", "1\nA\n2\nB\n3\n");
    write("b.txt", "1\nA\n2\nB\n3\n");

    let model = cx.new(|cx| RepoModel::new(gd_git::Repository::open(repo).unwrap(), cx));
    cx.run_until_parked();
    // Deselect "B" in both files, leaving `a.txt` on screen.
    for ix in [1, 0] {
        model.update(cx, |m, cx| m.select_change(ix, cx));
        cx.run_until_parked();
        model.update(cx, |m, cx| {
            let diff = m.work.detail.diff.clone().unwrap();
            let row = diff.lines.iter().position(|l| l.text == "B").unwrap();
            assert!(m.set_rows_included([row], false, cx));
        });
    }

    // An agent adds lines around the selection in both files.
    write("a.txt", "0\n1\nA\n2\nB\n3\nC\n");
    write("b.txt", "0\n1\nA\n2\nB\n3\nC\n");
    model.update(cx, |m, cx| m.refresh(cx));
    cx.run_until_parked();

    model.read_with(cx, |m, _| {
        let staged: Vec<(String, String)> = m
            .partial_files()
            .into_iter()
            .map(|f| (f.path, f.content))
            .collect();
        let want = "0\n1\nA\n2\n3\nC\n".to_string();
        assert_eq!(
            staged,
            [("a.txt".into(), want.clone()), ("b.txt".into(), want)]
        );
        // The gutter of the shown file agrees with what gets staged.
        let diff = m.work.detail.diff.clone().unwrap();
        for (ix, l) in diff.lines.iter().enumerate() {
            if gd_git::selectable(&diff, ix) {
                assert_eq!(m.row_included(ix), l.text != "B", "{}", l.text);
            }
        }
    });
}
