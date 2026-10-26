//! Search over the loaded commits (History tab search box).

use gd_git::Commit;

/// Indices into `commits` that match `query`: every whitespace-separated word
/// must appear in the summary, body, author or co-authors, or be a prefix of the sha (case-insensitive).
/// `None` when the query is blank, meaning "show everything".
pub fn matching_commits(commits: &[Commit], query: &str) -> Option<Vec<usize>> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return None;
    }
    let words: Vec<&str> = query.split_whitespace().collect();
    let matches = commits
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            let hay = haystack(c);
            words.iter().all(|w| hay.contains(w) || c.sha.starts_with(w))
        })
        .map(|(i, _)| i)
        .collect();
    Some(matches)
}

fn haystack(c: &Commit) -> String {
    let mut s = format!("{}\n{}\n{}\n{}", c.summary, c.body, c.author_name, c.author_email);
    for a in &c.co_authors {
        s.push('\n');
        s.push_str(&a.name);
    }
    s.to_lowercase()
}
