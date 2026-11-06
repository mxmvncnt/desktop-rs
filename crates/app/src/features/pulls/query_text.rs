//! The state tabs write into the query, like GitHub's: picking one replaces the state terms of the text.

use gd_forge::PullState;

/// What the page starts with.
pub const DEFAULT_QUERY: &str = "is:pr state:open";

/// `query` with its top-level state terms replaced by the ones of `state` (`None` = all).
pub fn with_state(query: &str, state: Option<PullState>) -> String {
    let is_state = |t: &str| {
        t.starts_with("state:")
            || matches!(t, "is:open" | "is:closed" | "is:merged" | "is:unmerged")
    };
    let mut depth = 0i32;
    let mut kept: Vec<&str> = Vec::new();
    for t in query.split_whitespace() {
        let top = depth == 0;
        depth += t.matches('(').count() as i32 - t.matches(')').count() as i32;
        if !(top && is_state(t)) {
            kept.push(t);
        }
    }
    let added: &[&str] = match state {
        Some(PullState::Open) => &["state:open"],
        Some(PullState::Merged) => &["is:merged"],
        Some(PullState::Closed) => &["state:closed", "is:unmerged"],
        None => &[],
    };
    // After `is:pr`, where GitHub puts it.
    let at = kept.iter().position(|t| *t == "is:pr").map_or(0, |i| i + 1);
    for (i, t) in added.iter().enumerate() {
        kept.insert(at + i, t);
    }
    kept.join(" ")
}

#[cfg(test)]
#[path = "tests/query_text.test.rs"]
mod tests;
