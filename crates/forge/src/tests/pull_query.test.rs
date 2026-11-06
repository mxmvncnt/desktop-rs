use super::*;
use crate::model::User;

const EXAMPLE: &str = "is:pr state:open archived:false sort:updated-desc (involves:@me OR user-review-requested:@me) -author:@me";

fn pull(author: &str, reviewers: &[&str], state: PullState) -> PullRequest {
    PullRequest {
        number: 1,
        title: "Speed up the parser".into(),
        body: "Uses a table.".into(),
        state,
        draft: false,
        author: Some(User {
            name: None,
            username: author.into(),
            avatar_url: None,
        }),
        head_ref: Some("fast".into()),
        head_sha: None,
        base_ref: Some("main".into()),
        from_fork: false,
        labels: vec!["perf".into()],
        assignees: Vec::new(),
        reviewers: reviewers.iter().map(|r| r.to_string()).collect(),
        comments: None,
        created_at: None,
        updated_at: None,
        web_url: String::new(),
        diff_refs: None,
    }
}

#[test]
fn parses_the_example() {
    let q = PullQuery::parse(EXAMPLE).unwrap();
    assert_eq!(q.sort, Some((SortKey::Updated, true)));
    assert_eq!(q.state(), Some(PullState::Open));
    assert!(!q.only_state());
    let Expr::All(terms) = &q.expr else {
        panic!("{:?}", q.expr)
    };
    assert_eq!(terms.len(), 5, "sort is taken out: {terms:?}");
    assert_eq!(
        terms[3],
        Expr::Any(vec![
            Expr::All(vec![Expr::Qualifier("involves".into(), "@me".into())]),
            Expr::All(vec![Expr::Qualifier(
                "user-review-requested".into(),
                "@me".into()
            )]),
        ])
    );
    assert_eq!(
        terms[4],
        Expr::Not(Box::new(Expr::Qualifier("author".into(), "@me".into())))
    );
    assert!(q.unsupported().is_empty(), "{:?}", q.unsupported());
}

#[test]
fn evaluates_the_example() {
    let q = PullQuery::parse(EXAMPLE).unwrap();
    // Someone else's, asking me for a review: in.
    assert!(q.matches(&pull("grace", &["ada"], PullState::Open), "ada"));
    // Mine: out (-author:@me).
    assert!(!q.matches(&pull("ada", &["ada"], PullState::Open), "ada"));
    // Not involving me: out.
    assert!(!q.matches(&pull("grace", &["linus"], PullState::Open), "ada"));
    // Merged: out (state:open).
    assert!(!q.matches(&pull("grace", &["ada"], PullState::Merged), "ada"));
}

#[test]
fn text_labels_quotes_and_states() {
    let p = pull("grace", &[], PullState::Merged);
    let m = |q: &str| PullQuery::parse(q).unwrap().matches(&p, "ada");
    assert!(m("parser"));
    assert!(m("\"the parser\" label:perf"));
    assert!(!m("label:docs"));
    assert!(m("state:closed"), "closed includes merged, like GitHub");
    assert!(m("is:merged -is:draft"));
    assert!(!m("is:unmerged"));
    assert!(m("head:fast base:main"));
    assert!(m("-(author:linus OR author:ada)"));
    assert!(
        m("fix-up OR parser"),
        "a hyphen inside a word is no negation"
    );
}

#[test]
fn only_a_state_uses_the_plain_list() {
    assert!(PullQuery::parse("is:pr state:closed").unwrap().only_state());
    assert!(
        PullQuery::parse("is:pr is:merged sort:created-asc")
            .unwrap()
            .only_state()
    );
    assert!(
        !PullQuery::parse("is:pr state:open label:x")
            .unwrap()
            .only_state()
    );
    assert_eq!(
        PullQuery::parse("sort:created-asc").unwrap().sort,
        Some((SortKey::Created, false))
    );
}

#[test]
fn reports_bad_queries_and_unknown_qualifiers() {
    assert!(PullQuery::parse("(a OR b").is_err());
    assert!(PullQuery::parse("a)").is_err());
    assert!(PullQuery::parse("\"open").is_err());
    let q = PullQuery::parse("is:pr milestone:v2 reactions:>5").unwrap();
    assert_eq!(q.unsupported(), ["milestone:v2", "reactions:>5"]);
}
