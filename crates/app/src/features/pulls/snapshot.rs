//! Snapshot views: the pull requests page filled with fixture data (nothing is fetched). The pull request's commits and
//! files are the repository's `feature` branch against `main` (see the test repo recipe in docs/PROGRESS.md).

use chrono::{Duration, Utc};
use gd_forge::{
    Comment, Conversation, Forge, PullQuery, PullRequest, PullState, RepoRef, Side, Thread, User,
};
use gpui::{Context, Window};

use super::state::{Pick, PullTab, PullsEvent, PullsStore, PullsView};
use crate::features::accounts::{Problem, Target};
use crate::features::diff::text_selection::model::{TextPos, TextSel, TextSide};
use crate::features::settings::ThemeMode;
use crate::snapshot::View;
use crate::workspace::Workspace;

pub fn views() -> &'static [View] {
    &[
        View {
            name: "pulls*",
            steps: &[pulls_sample],
        },
        View {
            name: "pull-files-select",
            steps: &[pulls_sample, select_text],
        },
        View {
            name: "pull-*",
            steps: &[pulls_sample],
        },
    ]
}

/// `pulls[-light]`: the list; `pulls-query`: the example query of docs/PROGRESS.md on a GitLab account (filtered here, as
/// `PullFilter::keep` would), with a qualifier GitLab can't filter on; `pulls-query-error`: a query that doesn't parse;
/// `pulls-empty-account`; `pull-conversation`, `pull-commits`, `pull-files`, `pull-files-comment`
/// (lines picked, the comment box open), `pull-files-split`, `pull-files-select` (text selected on README.md lines 4-6: the
/// "Comment on lines" bar). The fixture has a resolved and an outdated thread (minimized) and a Markdown comment.
fn pulls_sample(ws: &mut Workspace, view: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.settings.theme = if view.ends_with("light") {
        ThemeMode::Light
    } else {
        ThemeMode::Dark
    };
    ws.settings.diff_split = view.contains("split");
    ws.apply_theme(window, cx);
    let root = ws.repo.as_ref().map(|r| r.read(cx).repo.clone());
    ws.pulls.update(cx, |p, cx| {
        p.open = true;
        if view.starts_with("pulls-empty-account") {
            p.target = Some(Err(Problem::NoAccount("github.example.com".into())));
            return;
        }
        fill(p);
        if view.starts_with("pulls-query") {
            let text = if view.contains("error") {
                "is:pr (involves:@me OR"
            } else {
                "is:pr state:open archived:false sort:updated-desc (involves:@me OR user-review-requested:@me) -author:@me milestone:v2"
            };
            if let Some(Ok(t)) = &mut p.target {
                t.forge = Forge::GitLab;
            }
            p.inputs.query.update(cx, |s, cx| s.emplace(text, cx));
            match PullQuery::parse(text) {
                Ok(q) => {
                    p.pulls.retain(|pull| q.matches(pull, "ada"));
                    p.query = q;
                }
                Err(e) => p.query_error = Some(e),
            }
        }
        let Some(tab) = [
            ("pull-conversation", PullTab::Conversation),
            ("pull-commits", PullTab::Commits),
            ("pull-files", PullTab::Files),
        ]
        .into_iter()
        .find_map(|(prefix, tab)| view.starts_with(prefix).then_some(tab)) else {
            return;
        };
        p.view = PullsView::Pull(42);
        p.detail = p.pulls.first().cloned();
        p.conversation = Some(conversation());
        p.head = Some(
            root.ok_or("no repository".to_string())
                .and_then(|r| r.pull_head("feature", "main").map_err(|e| e.to_string())),
        );
        p.tab = tab;
        if tab == PullTab::Commits {
            p.commit = Some(0);
        }
        if view.contains("comment") {
            p.pick = Some(Pick {
                path: "README.md".into(),
                start: 4,
                end: 6,
            });
        }
        if let Some(head) = p.pull_head() {
            let (base, head) = match tab {
                PullTab::Commits => (
                    head.commits[0].parents[0].clone(),
                    head.commits[0].sha.clone(),
                ),
                _ => (head.merge_base.clone(), head.head.clone()),
            };
            cx.emit(PullsEvent::Review { base, head });
        }
        cx.notify();
    });
}

/// `pull-files-select`: a finished text selection over the diff lines 4 to 6, once the diff is loaded.
fn select_text(ws: &mut Workspace, _: &str, _: &mut Window, cx: &mut Context<Workspace>) {
    ws.diff_view.text_sel = Some(TextSel {
        anchor: TextPos { line: 4, col: 0 },
        head: TextPos { line: 6, col: 4 },
        side: TextSide::Unified,
    });
    cx.notify();
}

fn user(name: &str) -> Option<User> {
    Some(User {
        name: None,
        username: name.into(),
        avatar_url: None,
    })
}

fn fill(p: &mut PullsStore) {
    p.target = Some(Ok(Target {
        forge: Forge::GitHub,
        repo: RepoRef {
            host: "github.com".into(),
            owner: "octo".into(),
            name: "repo".into(),
        },
        root: Default::default(),
        branch: Some("main".into()),
        account_id: "a".into(),
        account_name: "Personal".into(),
        username: "ada".into(),
        host: "github.com".into(),
    }));
    let now = Utc::now();
    let pull = |number,
                title: &str,
                state,
                draft,
                author: &str,
                head: &str,
                hours,
                labels: &[&str],
                comments| {
        PullRequest {
        number,
        title: title.into(),
        body: "Adds a README so newcomers know what the modules are for.\n\nAlso greets the world properly.".into(),
        state,
        draft,
        author: user(author),
        head_ref: Some(head.into()),
        head_sha: None,
        base_ref: Some("main".into()),
        from_fork: number == 39,
        labels: labels.iter().map(|l| l.to_string()).collect(),
        assignees: Vec::new(),
        reviewers: if number == 41 { vec!["ada".into()] } else { Vec::new() },
        comments: Some(comments),
        created_at: Some(now - Duration::hours(hours + 30)),
        updated_at: Some(now - Duration::hours(hours)),
        web_url: format!("https://github.com/octo/repo/pull/{number}"),
        diff_refs: None,
        milestone: None,
        merge: None,
    }
    };
    p.pulls = vec![
        pull(
            42,
            "Add a README and greet the world",
            PullState::Open,
            false,
            "ada",
            "feature",
            2,
            &["docs"],
            3,
        ),
        pull(
            41,
            "Speed up the diff parser",
            PullState::Open,
            true,
            "grace",
            "fast-diff",
            5,
            &["perf", "wip"],
            0,
        ),
        pull(
            40,
            "Fix the toolbar on macOS",
            PullState::Merged,
            false,
            "linus",
            "fix/toolbar",
            26,
            &[],
            7,
        ),
        pull(
            39,
            "Translate the settings page",
            PullState::Open,
            false,
            "margaret",
            "i18n",
            50,
            &["i18n"],
            1,
        ),
        pull(
            38,
            "Remove the old welcome screen",
            PullState::Closed,
            false,
            "ada",
            "old-welcome",
            120,
            &[],
            2,
        ),
    ];
}

fn conversation() -> Conversation {
    let now = Utc::now();
    let comment = |id, author: &str, body: &str, hours| Comment {
        id,
        author: user(author),
        body: body.into(),
        created_at: Some(now - Duration::hours(hours)),
        verdict: None,
    };
    Conversation {
        comments: vec![
            comment(1, "grace", "Thanks! The README reads well.", 20),
            Comment {
                verdict: Some("approved".into()),
                ..comment(2, "linus", "", 3)
            },
        ],
        threads: vec![
            Thread {
                id: "12".into(),
                path: "README.md".into(),
                side: Side::Right,
                line: None,
                start_line: None,
                original_line: Some(2),
                resolved: false,
                comments: vec![comment(
                    12,
                    "linus",
                    "This heading moved, is the title still right?",
                    30,
                )],
            },
            Thread {
                id: "13".into(),
                path: "README.md".into(),
                side: Side::Right,
                line: Some(5),
                start_line: None,
                original_line: Some(5),
                resolved: true,
                comments: vec![
                    comment(13, "grace", "Typo here.", 12),
                    comment(14, "ada", "Fixed.", 11),
                ],
            },
            Thread {
                id: "10".into(),
                path: "README.md".into(),
                side: Side::Right,
                line: Some(3),
                start_line: None,
                original_line: Some(3),
                resolved: false,
                comments: vec![
                    comment(10, "grace", "Maybe say what it is for?", 10),
                    comment(
                        11,
                        "ada",
                        "Good idea, **done** in `README.md`:\n\n- greets the world\n- links to the [docs](https://example.com)\n\n```suggestion\nHello, world!\n```",
                        8,
                    ),
                ],
            },
        ],
        reviews: Vec::new(),
    }
}
