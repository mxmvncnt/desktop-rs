use std::path::Path;
use std::process::Command;

/// Scratch directory inside `target/`, removed on drop.
pub fn tempdir() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap()
}

#[allow(dead_code)] // not every test file runs git directly
/// Run `git` in `dir`, assert success, return trimmed stdout.
pub fn sh(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git").current_dir(dir).env("LC_ALL", "C").args(args).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}
