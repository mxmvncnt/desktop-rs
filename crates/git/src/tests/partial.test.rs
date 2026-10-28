use super::*;
use crate::DiffOptions;
use crate::engine::diff_texts;

fn stage(old: &str, new: &str, pick: impl Fn(&crate::DiffLine) -> bool) -> String {
    let d = diff_texts(old, new, DiffOptions::default());
    staged_text(&d, |ix| pick(&d.lines[ix])).unwrap()
}

#[test]
fn all_and_none() {
    let (old, new) = ("a\nb\nc\n", "a\nB\nc\nd\n");
    assert_eq!(stage(old, new, |_| true), new);
    assert_eq!(stage(old, new, |_| false), old);
}

#[test]
fn only_adds_or_only_deletes() {
    let (old, new) = ("a\nb\nc\n", "a\nB\nc\nd\n");
    assert_eq!(stage(old, new, |l| l.kind == DiffLineKind::Add), "a\nb\nB\nc\nd\n");
    assert_eq!(stage(old, new, |l| l.kind == DiffLineKind::Delete), "a\nc\n");
}

#[test]
fn one_line_far_apart() {
    let old: String = (1..=30).map(|i| format!("{i}\n")).collect();
    let new = old.replace("2\n", "two\n").replace("28\n", "twenty-eight\n");
    let got = stage(&old, &new, |l| l.text == "twenty-eight" || l.text == "28");
    assert_eq!(got, old.replace("28\n", "twenty-eight\n"));
}

#[test]
fn missing_final_newline() {
    assert_eq!(stage("a", "a\nb\n", |l| l.text == "b"), "a\nb\n");
    assert_eq!(stage("", "x\ny", |l| l.text == "x"), "x\n");
    assert_eq!(stage("", "x\ny", |l| l.text == "y"), "y");
}

#[test]
fn crlf_kept() {
    assert_eq!(stage("a\r\nb\r\n", "a\r\nB\r\n", |l| l.kind == DiffLineKind::Add), "a\r\nb\r\nB\r\n");
}

#[test]
fn discard_rows_reverts_only_picked_lines() {
    let (old, new) = ("a\nb\nc\n", "a\nB\nc\nd\n");
    // Reverting just the added "d" keeps the b -> B edit.
    let d = diff_texts(old, new, DiffOptions::default());
    let rows: Vec<usize> = (0..d.lines.len()).filter(|&i| d.lines[i].text == "d").collect();
    assert_eq!(staged_text(&d, |ix| !rows.contains(&ix)).unwrap(), "a\nB\nc\n");
}
