use super::*;
use gd_git::DiffLine;

fn line(kind: DiffLineKind, old: Option<u32>, new: Option<u32>) -> DiffLine {
    DiffLine {
        kind,
        old_line: old,
        new_line: new,
        text: String::new(),
        emphasis: Vec::new(),
    }
}

fn thread(side: Side, line: Option<u32>) -> Thread {
    Thread {
        id: "1".into(),
        path: "a.rs".into(),
        side,
        line,
        start_line: None,
        original_line: line,
        resolved: false,
        comments: Vec::new(),
    }
}

#[test]
fn threads_anchor_on_their_side() {
    let diff = Diff {
        lines: vec![
            line(DiffLineKind::Hunk, None, None),
            line(DiffLineKind::Context, Some(4), Some(4)),
            line(DiffLineKind::Delete, Some(5), None),
            line(DiffLineKind::Add, None, Some(5)),
        ],
        ..Default::default()
    };
    // The new line 5 is the added one, the old line 5 the deleted one; a context line by its new number.
    assert_eq!(anchor(&diff, &thread(Side::Right, Some(5))), Some(3));
    assert_eq!(anchor(&diff, &thread(Side::Left, Some(5))), Some(2));
    assert_eq!(anchor(&diff, &thread(Side::Right, Some(4))), Some(1));
    // Outdated, or outside the hunks: nowhere in the diff.
    assert_eq!(anchor(&diff, &thread(Side::Right, None)), None);
    assert_eq!(anchor(&diff, &thread(Side::Right, Some(40))), None);
}
