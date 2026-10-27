//! Pure string helpers: plurals, paths, branch names, authors, relative time.

use std::time::{SystemTime, UNIX_EPOCH};

use gd_git::Commit;

/// English plural suffix for counts.
pub fn plural(n: u32) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// Split `a/b/c.rs` into (`a/b/`, `c.rs`) for the dim-dir / bright-name rows.
pub fn split_path(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(i) => (&path[..=i], &path[i + 1..]),
        None => ("", path),
    }
}

/// Desktop-style branch name sanitizing: spaces → `-`, strip invalid chars.
pub fn sanitize_branch_name(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.trim().chars() {
        match ch {
            c if c.is_whitespace() => out.push('-'),
            '~' | '^' | ':' | '?' | '*' | '[' | '\\' => {}
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    while out.contains("..") {
        out = out.replace("..", ".");
    }
    out.trim_matches(['/', '.', '-']).to_string()
}

/// Author plus co-authors: "A", "A and B", "A, B and C", "A, B and 2 others".
pub fn authors_text(c: &Commit) -> String {
    let mut names: Vec<&str> = vec![c.author_name.as_str()];
    names.extend(c.co_authors.iter().map(|a| a.name.as_str()));
    match names.as_slice() {
        [one] => (*one).into(),
        [a, b] => format!("{a} and {b}"),
        [a, b, c] => format!("{a}, {b} and {c}"),
        [a, b, rest @ ..] => format!("{a}, {b} and {} others", rest.len()),
        [] => String::new(),
    }
}

/// The commit body without `Co-authored-by:` trailer lines (shown as authors instead).
pub fn body_without_trailers(body: &str) -> String {
    let kept: Vec<&str> = body
        .lines()
        .filter(|l| !l.split_once(':').is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case("co-authored-by")))
        .collect();
    kept.join("\n").trim().to_string()
}


/// Largest unit used before the count just keeps growing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Coarsest {
    /// "45 days ago" (the Fetch caption).
    Day,
    /// "2 months ago", "3 years ago" (commit/branch lists).
    Year,
}

/// Relative time from unix seconds.
pub fn relative_time(unix: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_ago(now - unix, Coarsest::Year)
}

/// Relative time for a `SystemTime` (future times count as "just now").
pub fn relative_time_since(time: SystemTime, coarsest: Coarsest) -> String {
    let secs = time.elapsed().map(|d| d.as_secs() as i64).unwrap_or(0);
    format_ago(secs, coarsest)
}

fn format_ago(secs: i64, coarsest: Coarsest) -> String {
    let d = secs.max(0);
    let (n, unit) = match d {
        0..=59 => return "just now".into(),
        60..=3599 => (d / 60, "minute"),
        3600..=86_399 => (d / 3600, "hour"),
        _ if coarsest == Coarsest::Day => (d / 86_400, "day"),
        86_400..=2_591_999 => (d / 86_400, "day"),
        2_592_000..=31_535_999 => (d / 2_592_000, "month"),
        _ => (d / 31_536_000, "year"),
    };
    format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(format_ago(-5, Coarsest::Year), "just now");
        assert_eq!(format_ago(60, Coarsest::Year), "1 minute ago");
        assert_eq!(format_ago(7200, Coarsest::Year), "2 hours ago");
        assert_eq!(format_ago(86_400 * 45, Coarsest::Day), "45 days ago");
        assert_eq!(format_ago(86_400 * 45, Coarsest::Year), "1 month ago");
        assert_eq!(format_ago(31_536_000 * 3, Coarsest::Year), "3 years ago");
    }
}
