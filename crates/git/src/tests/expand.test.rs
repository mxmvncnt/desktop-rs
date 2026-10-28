use super::*;
use crate::DiffOptions;
use crate::engine::diff_texts;

fn file(n: usize) -> String {
    (1..=n).map(|i| format!("{i}\n")).collect()
}

/// Context rows must always show the line they claim to be.
fn check_rows(d: &Diff, new: &str) {
    let lines: Vec<&str> = new.lines().collect();
    for l in &d.lines {
        if l.kind == DiffLineKind::Context {
            assert_eq!(l.text, lines[l.new_line.unwrap() as usize - 1]);
        }
    }
}

#[test]
fn up_down_and_merge() {
    let old = file(100);
    let new = old.replace("10\n", "ten\n").replace("60\n", "sixty\n");
    let d = diff_texts(&old, &new, DiffOptions::default());
    assert_eq!(d.hunks.len(), 2);
    assert_eq!(gap_above(&d, 0), 6); // lines 1..=6 hidden
    assert_eq!(gap_above(&d, 1), 43); // lines 14..=56 hidden
    assert_eq!(gap_below(&d, 1), 100 - 63);

    let up = expand(&d, Expansion::Up(0)).unwrap();
    assert_eq!(gap_above(&up, 0), 0);
    assert_eq!(up.lines[0].text, "@@ -1,13 +1,13 @@");
    check_rows(&up, &new);

    let down = expand(&d, Expansion::Down(1)).unwrap();
    assert_eq!(gap_below(&down, 1), 37 - 20);
    check_rows(&down, &new);

    let all = expand(&d, Expansion::All(1)).unwrap();
    assert_eq!(all.hunks.len(), 1);
    assert_eq!(all.lines.iter().filter(|l| l.kind == DiffLineKind::Hunk).count(), 1);
    check_rows(&all, &new);
    // Changes and staging are unaffected by expansion.
    assert_eq!((all.added, all.deleted), (d.added, d.deleted));
    let pick = |d: &Diff| crate::staged_text(d, |ix| d.lines[ix].text == "ten" || d.lines[ix].text == "10").unwrap();
    assert_eq!(pick(&all), pick(&d));
}

#[test]
fn down_to_eof_without_newline() {
    let old = "a\nb\nc\nd\ne\nf\ng";
    let new = "A\nb\nc\nd\ne\nf\ng";
    let d = diff_texts(old, new, DiffOptions::default());
    let down = expand(&d, Expansion::Down(0)).unwrap();
    assert_eq!(down.lines.last().unwrap().kind, DiffLineKind::NoNewline);
    assert_eq!(gap_below(&down, 0), 0);
    assert!(expand(&down, Expansion::Down(0)).is_none());
}
