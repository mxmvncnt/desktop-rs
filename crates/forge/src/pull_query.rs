//! GitHub's pull request search syntax: `is:pr state:open sort:updated-desc (involves:@me OR review-requested:@me) -author:@me`.
//! GitHub runs the query itself; for the other forges `matches` evaluates it on the loaded pull requests.

use crate::pull_model::{PullRequest, PullState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// Every one (terms side by side, or `AND`).
    All(Vec<Expr>),
    /// `a OR b`.
    Any(Vec<Expr>),
    /// `-term`, `-(…)`.
    Not(Box<Expr>),
    /// `key:value`, key lowercase.
    Qualifier(String, String),
    /// A word or a quoted phrase, matched in the title and description.
    Text(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Created,
    Updated,
    Comments,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullQuery {
    /// The text as typed.
    pub raw: String,
    pub expr: Expr,
    /// `sort:updated-desc`; `None` = most recently updated first. Taken out of `expr`.
    pub sort: Option<(SortKey, bool)>,
}

/// What `matches` can't tell from a pull request, so it treats it as matching.
const IGNORED: [&str; 3] = ["archived", "repo", "org"];

impl PullQuery {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let tokens = tokenize(raw)?;
        let mut pos = 0;
        let mut expr = parse_any(&tokens, &mut pos)?;
        if pos < tokens.len() {
            return Err("unbalanced parenthesis".into());
        }
        let mut sort = None;
        if let Expr::All(terms) = &mut expr {
            terms.retain(|t| match t {
                Expr::Qualifier(k, v) if k == "sort" => {
                    sort = parse_sort(v);
                    false
                }
                _ => true,
            });
        }
        Ok(Self {
            raw: raw.trim().to_string(),
            expr,
            sort,
        })
    }

    /// The state the query asks for at its top level (`state:open`, `is:merged`, …), which the server can filter on.
    pub fn state(&self) -> Option<PullState> {
        self.top().find_map(|t| match t {
            Expr::Qualifier(k, v) => match (k.as_str(), v.as_str()) {
                ("state" | "is", "open") => Some(PullState::Open),
                ("is", "merged") => Some(PullState::Merged),
                ("state" | "is", "closed") => Some(PullState::Closed),
                _ => None,
            },
            _ => None,
        })
    }

    /// Nothing but a state (and `is:pr`, `sort:`): the plain list endpoint can answer it.
    pub fn only_state(&self) -> bool {
        self.top().all(|t| match t {
            Expr::Qualifier(k, v) => {
                matches!(
                    (k.as_str(), v.as_str()),
                    ("is", "pr" | "open" | "closed" | "merged") | ("state", _)
                ) || IGNORED.contains(&k.as_str())
            }
            _ => false,
        })
    }

    /// `author:<login>` at the top level, with `@me` resolved.
    pub fn author(&self, me: &str) -> Option<String> {
        self.top().find_map(|t| match t {
            Expr::Qualifier(k, v) if k == "author" => Some(resolve(v, me).to_string()),
            _ => None,
        })
    }

    /// Qualifiers `matches` does not know: they are treated as matching.
    pub fn unsupported(&self) -> Vec<String> {
        let mut out = Vec::new();
        collect_unsupported(&self.expr, &mut out);
        out
    }

    /// Whether `p` is one of the results; `me` is the login `@me` stands for.
    pub fn matches(&self, p: &PullRequest, me: &str) -> bool {
        eval(&self.expr, p, me)
    }

    fn top(&self) -> impl Iterator<Item = &Expr> {
        match &self.expr {
            Expr::All(terms) => terms.iter(),
            other => std::slice::from_ref(other).iter(),
        }
    }
}

#[derive(Debug, PartialEq)]
enum Token {
    Open,
    Close,
    Or,
    And,
    Minus,
    Word(String),
}

fn tokenize(s: &str) -> Result<Vec<Token>, String> {
    let mut out = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            c if c.is_whitespace() => {
                chars.next();
            }
            '(' => {
                chars.next();
                out.push(Token::Open);
            }
            ')' => {
                chars.next();
                out.push(Token::Close);
            }
            '-' => {
                chars.next();
                out.push(Token::Minus);
            }
            _ => {
                let mut word = String::new();
                let mut quoted = false;
                while let Some(&c) = chars.peek() {
                    if c == '"' {
                        quoted = !quoted;
                        chars.next();
                    } else if !quoted && (c.is_whitespace() || c == '(' || c == ')') {
                        break;
                    } else {
                        word.push(c);
                        chars.next();
                    }
                }
                if quoted {
                    return Err("unclosed quote".into());
                }
                out.push(match word.as_str() {
                    "OR" => Token::Or,
                    "AND" => Token::And,
                    _ => Token::Word(word),
                });
            }
        }
    }
    Ok(out)
}

fn parse_any(t: &[Token], pos: &mut usize) -> Result<Expr, String> {
    let mut any = vec![parse_all(t, pos)?];
    while t.get(*pos) == Some(&Token::Or) {
        *pos += 1;
        any.push(parse_all(t, pos)?);
    }
    Ok(if any.len() == 1 {
        any.remove(0)
    } else {
        Expr::Any(any)
    })
}

fn parse_all(t: &[Token], pos: &mut usize) -> Result<Expr, String> {
    let mut all = Vec::new();
    while let Some(token) = t.get(*pos) {
        match token {
            Token::Or | Token::Close => break,
            Token::And => *pos += 1,
            _ => all.push(parse_one(t, pos)?),
        }
    }
    Ok(Expr::All(all))
}

fn parse_one(t: &[Token], pos: &mut usize) -> Result<Expr, String> {
    let token = t.get(*pos).ok_or("the query ends too early")?;
    *pos += 1;
    match token {
        Token::Minus => Ok(Expr::Not(Box::new(parse_one(t, pos)?))),
        Token::Open => {
            let inner = parse_any(t, pos)?;
            if t.get(*pos) != Some(&Token::Close) {
                return Err("unbalanced parenthesis".into());
            }
            *pos += 1;
            Ok(inner)
        }
        Token::Word(w) => Ok(match w.split_once(':') {
            Some((k, v)) if !k.is_empty() && !v.is_empty() => {
                Expr::Qualifier(k.to_lowercase(), v.to_string())
            }
            _ => Expr::Text(w.clone()),
        }),
        Token::Close | Token::Or | Token::And => Err("misplaced OR, AND or parenthesis".into()),
    }
}

fn parse_sort(v: &str) -> Option<(SortKey, bool)> {
    let (key, dir) = v.rsplit_once('-').unwrap_or((v, "desc"));
    let key = match key {
        "created" => SortKey::Created,
        "updated" => SortKey::Updated,
        "comments" => SortKey::Comments,
        _ => return None,
    };
    Some((key, dir != "asc"))
}

fn resolve<'a>(v: &'a str, me: &'a str) -> &'a str {
    if v == "@me" { me } else { v }
}

fn known(key: &str, value: &str) -> bool {
    match key {
        "is" => matches!(
            value,
            "pr" | "open" | "closed" | "merged" | "unmerged" | "draft"
        ),
        "state"
        | "author"
        | "assignee"
        | "label"
        | "involves"
        | "review-requested"
        | "user-review-requested"
        | "draft"
        | "head"
        | "base"
        | "in" => true,
        _ => IGNORED.contains(&key),
    }
}

fn collect_unsupported(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::All(v) | Expr::Any(v) => v.iter().for_each(|e| collect_unsupported(e, out)),
        Expr::Not(e) => collect_unsupported(e, out),
        Expr::Qualifier(k, v) if !known(k, v) => out.push(format!("{k}:{v}")),
        _ => {}
    }
}

fn eval(e: &Expr, p: &PullRequest, me: &str) -> bool {
    match e {
        Expr::All(v) => v.iter().all(|e| eval(e, p, me)),
        Expr::Any(v) => v.iter().any(|e| eval(e, p, me)),
        Expr::Not(e) => !eval(e, p, me),
        Expr::Text(t) => {
            let t = t.to_lowercase();
            p.title.to_lowercase().contains(&t) || p.body.to_lowercase().contains(&t)
        }
        Expr::Qualifier(k, v) => qualifier(k, v, p, me),
    }
}

fn qualifier(key: &str, value: &str, p: &PullRequest, me: &str) -> bool {
    let login = resolve(value, me);
    let is = |user: &str| user.eq_ignore_ascii_case(login);
    let author = p.author.as_ref().is_some_and(|u| is(&u.username));
    let reviewer = p.reviewers.iter().any(|r| is(r));
    match (key, value) {
        ("is", "pr") => true,
        ("is" | "state", "open") => p.state == PullState::Open,
        ("is" | "state", "closed") => p.state != PullState::Open,
        ("is", "merged") => p.state == PullState::Merged,
        ("is", "unmerged") => p.state != PullState::Merged,
        ("is", "draft") => p.draft,
        ("draft", v) => p.draft == (v == "true"),
        ("author", _) => author,
        ("assignee", _) => p.assignees.iter().any(|a| is(a)),
        ("review-requested" | "user-review-requested", _) => reviewer,
        // GitHub also counts commenters and mentions; the list knows only these.
        ("involves", _) => author || reviewer || p.assignees.iter().any(|a| is(a)),
        ("label", v) => v.split(',').any(|v| {
            p.labels
                .iter()
                .any(|l| l.eq_ignore_ascii_case(v.trim_matches('"')))
        }),
        ("head", v) => p.head_ref.as_deref() == Some(v),
        ("base", v) => p.base_ref.as_deref() == Some(v),
        _ => true,
    }
}

#[cfg(test)]
#[path = "tests/pull_query.test.rs"]
mod tests;
