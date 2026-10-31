//! Job logs: ANSI colors into flat styled spans, collapsible sections (GitLab `section_start`/`section_end`,
//! GitHub `##[group]`/`##[endgroup]`), GitHub's leading timestamps, `\r` overwrites.

use std::ops::Range;

use chrono::{DateTime, Utc};
use vte::{Params, Parser, Perform};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// 0-7 normal, 8-15 bright, 16-255 the xterm palette.
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

/// A run of `Line::text` with one style. A line's spans cover its whole text, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub style: Style,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Line {
    pub text: String,
    pub spans: Vec<Span>,
    /// GitHub and Gitea prefix every line with a timestamp.
    pub time: Option<DateTime<Utc>>,
}

/// Lines `header + 1 .. end` are the body; the header line (index `header`) stays visible when collapsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub header: usize,
    /// One past the last line of the section.
    pub end: usize,
    /// Seconds, when the log says so (GitLab).
    pub duration: Option<u64>,
    pub collapsed: bool,
    /// 0 for top-level sections.
    pub depth: usize,
}

/// Sections are in the order they start.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Log {
    pub lines: Vec<Line>,
    pub sections: Vec<Section>,
}

/// A section that has started and not ended yet.
struct Open {
    index: usize,
    started: Option<i64>,
}

pub fn parse_log(raw: &str) -> Log {
    let mut log = Log::default();
    let mut open: Vec<Open> = Vec::new();
    let mut painter = Painter::default();
    for line in raw.trim_start_matches('\u{feff}').lines() {
        let (time, line) = split_timestamp(line);
        let mut content = "";
        let mut marked = false;
        // `\r` rewinds the line: only the last non-empty segment stays. GitLab puts its markers in their own segments.
        for segment in line.split('\r') {
            let segment = segment.strip_prefix("\x1b[0K").unwrap_or(segment);
            if let Some(marker) = segment.strip_prefix("section_start:") {
                let (started, name) = marker.split_once(':').unwrap_or(("", marker));
                let (name, options) = name.split_once('[').unwrap_or((name, ""));
                let collapsed = options.split([',', ']']).any(|o| o == "collapsed=true");
                start_section(&mut log, &mut open, name, started.parse().ok(), collapsed);
                marked = true;
            } else if let Some(marker) = segment.strip_prefix("section_end:") {
                let (ended, name) = marker.split_once(':').unwrap_or(("", marker));
                if let Some(at) = open.iter().rposition(|o| log.sections[o.index].name == name) {
                    end_section(&mut log, open.remove(at), ended.parse().ok());
                }
                marked = true;
            } else if !segment.is_empty() {
                content = segment;
            }
        }
        if let Some(title) = content.strip_prefix("##[group]") {
            start_section(&mut log, &mut open, title, None, true);
            content = title;
        } else if content == "##[endgroup]" {
            if let Some(top) = open.pop() {
                end_section(&mut log, top, None);
            }
            continue;
        } else if content.is_empty() && marked {
            continue;
        }
        painter.paint(content);
        log.lines.push(Line { text: std::mem::take(&mut painter.text), spans: std::mem::take(&mut painter.spans), time });
    }
    for unclosed in open {
        end_section(&mut log, unclosed, None);
    }
    log
}

/// A section starts at the next line pushed, which is its header.
fn start_section(log: &mut Log, open: &mut Vec<Open>, name: &str, started: Option<i64>, collapsed: bool) {
    log.sections.push(Section { name: name.to_string(), header: log.lines.len(), end: 0, duration: None, collapsed, depth: open.len() });
    open.push(Open { index: log.sections.len() - 1, started });
}

fn end_section(log: &mut Log, section: Open, ended: Option<i64>) {
    let s = &mut log.sections[section.index];
    s.end = log.lines.len();
    s.duration = ended.zip(section.started).and_then(|(e, s)| u64::try_from(e - s).ok());
}

fn split_timestamp(line: &str) -> (Option<DateTime<Utc>>, &str) {
    let Some((token, rest)) = line.split_once(' ') else { return (None, line) };
    let time = (token.len() >= 20 && token.ends_with('Z') && token.as_bytes()[4] == b'-')
        .then(|| DateTime::parse_from_rfc3339(token).ok())
        .flatten();
    match time {
        Some(t) => (Some(t.to_utc()), rest),
        None => (None, line),
    }
}

/// Turns SGR sequences into spans and drops every other control sequence.
#[derive(Default)]
struct Painter {
    style: Style,
    text: String,
    spans: Vec<Span>,
}

impl Painter {
    fn paint(&mut self, content: &str) {
        Parser::new().advance(self, content.as_bytes());
    }

    fn sgr(&mut self, codes: &[u16]) {
        let mut codes = codes.iter().copied();
        let style = &mut self.style;
        while let Some(code) = codes.next() {
            match code {
                0 => *style = Style::default(),
                1 => style.bold = true,
                3 => style.italic = true,
                4 => style.underline = true,
                22 => style.bold = false,
                23 => style.italic = false,
                24 => style.underline = false,
                30..=37 => style.fg = Some(Color::Indexed((code - 30) as u8)),
                90..=97 => style.fg = Some(Color::Indexed((code - 90 + 8) as u8)),
                40..=47 => style.bg = Some(Color::Indexed((code - 40) as u8)),
                100..=107 => style.bg = Some(Color::Indexed((code - 100 + 8) as u8)),
                39 => style.fg = None,
                49 => style.bg = None,
                38 | 48 => {
                    let color = match codes.next() {
                        Some(5) => codes.next().map(|n| Color::Indexed(n as u8)),
                        Some(2) => match (codes.next(), codes.next(), codes.next()) {
                            (Some(r), Some(g), Some(b)) => Some(Color::Rgb(r as u8, g as u8, b as u8)),
                            _ => None,
                        },
                        _ => None,
                    };
                    if code == 38 { style.fg = color } else { style.bg = color }
                }
                _ => {}
            }
        }
    }
}

impl Perform for Painter {
    fn print(&mut self, c: char) {
        let start = self.text.len();
        self.text.push(c);
        match self.spans.last_mut() {
            Some(last) if last.style == self.style => last.range.end = self.text.len(),
            _ => self.spans.push(Span { range: start..self.text.len(), style: self.style }),
        }
    }

    fn execute(&mut self, byte: u8) {
        if byte == b'\t' {
            self.print('\t');
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _: &[u8], _: bool, action: char) {
        if action == 'm' {
            let codes: Vec<u16> = params.iter().flatten().copied().collect();
            // `ESC[m` is a reset.
            self.sgr(if codes.is_empty() { &[0] } else { &codes });
        }
    }
}

#[cfg(test)]
#[path = "tests/log.test.rs"]
mod tests;
