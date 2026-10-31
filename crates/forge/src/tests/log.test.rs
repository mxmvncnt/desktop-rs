use super::*;

fn texts(log: &Log) -> Vec<&str> {
    log.lines.iter().map(|l| l.text.as_str()).collect()
}

#[test]
fn sgr_colors() {
    let log = parse_log("plain \x1b[1;31mred bold\x1b[0m \x1b[38;5;208morange\x1b[39m \x1b[48;2;1;2;3;4munder\x1b[m\n");
    let line = &log.lines[0];
    assert_eq!(line.text, "plain red bold orange under");
    let bold_red = Style { fg: Some(Color::Indexed(1)), bold: true, ..Style::default() };
    assert_eq!(line.spans[1], Span { range: 6..14, style: bold_red });
    assert_eq!(line.spans[3].style.fg, Some(Color::Indexed(208)));
    assert_eq!(line.spans[5].style, Style { bg: Some(Color::Rgb(1, 2, 3)), underline: true, ..Style::default() });
    let covered: usize = line.spans.iter().map(|s| s.range.len()).sum();
    assert_eq!(covered, line.text.len());
}

#[test]
fn bright_colors_and_style_carries_over_lines() {
    let log = parse_log("\x1b[92mgreen\nstill green\x1b[0m\n");
    assert_eq!(log.lines[1].spans[0].style.fg, Some(Color::Indexed(10)));
}

#[test]
fn other_control_sequences_are_stripped() {
    let log = parse_log("a\x1b[2Kb\x1b]0;title\x07c\x07d\n");
    assert_eq!(texts(&log), ["abcd"]);
}

#[test]
fn carriage_return_keeps_last_segment() {
    let log = parse_log("10%\r50%\r100%\r\nnext\r\n");
    assert_eq!(texts(&log), ["100%", "next"]);
}

#[test]
fn gitlab_sections() {
    let raw = "before\n\
        section_start:1700000000:prepare[collapsed=true]\r\x1b[0K\x1b[36;1mPreparing\x1b[0;m\n\
        step 1\n\
        section_start:1700000001:inner\r\x1b[0KInner header\n\
        in inner\n\
        section_end:1700000004:inner\r\x1b[0K\n\
        section_end:1700000010:prepare\r\x1b[0Ksection_start:1700000010:build\r\x1b[0KBuilding\n\
        compiling\n\
        section_end:1700000012:build\r\x1b[0K\n\
        after\n";
    let log = parse_log(raw);
    assert_eq!(texts(&log), ["before", "Preparing", "step 1", "Inner header", "in inner", "Building", "compiling", "after"]);
    let [prepare, inner, build] = &log.sections[..] else { panic!("{:?}", log.sections) };
    assert_eq!((prepare.name.as_str(), prepare.header, prepare.end, prepare.duration, prepare.collapsed, prepare.depth), ("prepare", 1, 5, Some(10), true, 0));
    assert_eq!((inner.header, inner.end, inner.duration, inner.collapsed, inner.depth), (3, 5, Some(3), false, 1));
    assert_eq!((build.header, build.end, build.duration), (5, 7, Some(2)));
    assert_eq!(log.lines[1].spans[0].style.fg, Some(Color::Indexed(6)));
}

#[test]
fn github_groups_and_timestamps() {
    let raw = "\u{feff}2025-03-01T10:00:00.1234567Z ##[group]Run cargo test\n\
        2025-03-01T10:00:01.0000000Z cargo test\n\
        2025-03-01T10:00:02.0000000Z ##[endgroup]\n\
        2025-03-01T10:00:03.0000000Z \x1b[32mok\x1b[0m\n\
        no timestamp\n";
    let log = parse_log(raw);
    assert_eq!(texts(&log), ["Run cargo test", "cargo test", "ok", "no timestamp"]);
    assert_eq!(log.sections, [Section { name: "Run cargo test".into(), header: 0, end: 2, duration: None, collapsed: true, depth: 0 }]);
    assert_eq!(log.lines[0].time, Some(DateTime::parse_from_rfc3339("2025-03-01T10:00:00.1234567Z").unwrap().to_utc()));
    assert!(log.lines[3].time.is_none());
}

#[test]
fn unclosed_section_runs_to_the_end() {
    let log = parse_log("##[group]Never closed\nx\ny\n");
    assert_eq!((log.sections[0].header, log.sections[0].end), (0, 3));
}
