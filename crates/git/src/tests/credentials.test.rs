use std::io::Write;
use std::process::{Command, Stdio};

use super::*;

/// Ask `git credential fill` for a credential with the env a network op gets.
#[test]
fn helper_answers_with_token() {
    let cred = HostCredential { host: "git.example.com".into(), username: "me".into(), token: "s3cret".into() };
    set_credentials(vec![cred]);
    let mut child = Command::new("git")
        .args(["credential", "fill"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .envs(env())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"protocol=https\nhost=git.example.com\npath=o/r.git\n\n").unwrap();
    let out = String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap();
    assert!(out.contains("username=me") && out.contains("password=s3cret"), "got {out:?}");
}
