//! `impl PullsStore`: resolving the account, loading the list and an open pull request, navigation and the comment requests.

use std::collections::HashSet;

use gd_forge::{Forge, LineTarget, PullFilter, PullRequest, PullState, Thread, pull_ref};
use gd_git::{PullCheckout, Repository};
use gpui::{AppContext, Context};

use super::state::{Pick, PullTab, PullsEvent, PullsStore, PullsView};
use crate::features::accounts::{Account, RepoAccounts, Target};
use crate::repo::spawn_load;

/// The forges answer 20 pull requests per page.
const PAGE_SIZE: usize = 20;

/// What a comment is on.
pub enum Reply {
    Conversation,
    Lines(LineTarget),
    Thread(Thread),
}

impl PullsStore {
    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        cx.notify();
    }

    /// Esc, the ← button and the mouse Back button: one screen back; from the list, close the page.
    pub fn back(&mut self, cx: &mut Context<Self>) {
        match self.view {
            PullsView::List => self.close(cx),
            PullsView::Pull(_) => {
                self.forward.push(self.view);
                self.view = PullsView::List;
            }
        }
        cx.notify();
    }

    /// The mouse Forward button: the screen Back left last.
    pub fn forward(&mut self, cx: &mut Context<Self>) {
        if let Some(PullsView::Pull(number)) = self.forward.pop() {
            self.show_pull(number, cx);
        }
    }

    /// Open the page for `repo` (or refresh it): another repository starts from scratch, the same one keeps what it loaded.
    pub fn show(
        &mut self,
        repo: Repository,
        accounts: Vec<Account>,
        picks: RepoAccounts,
        cx: &mut Context<Self>,
    ) {
        if self.root.as_deref() != Some(repo.root()) {
            let fresh = Self::new(cx);
            *self = Self {
                epoch: self.epoch + 1,
                root: Some(repo.root().to_path_buf()),
                inputs: std::mem::replace(&mut self.inputs, fresh.inputs),
                ..fresh
            };
        }
        self.open = true;
        self.refreshing = true;
        spawn_load(
            self,
            |s| &mut s.resolve_load,
            move || Target::resolve(&repo, None, &accounts, &picks),
            |this, target, cx| {
                let ready = target.is_ok();
                this.target = Some(target);
                if !ready {
                    this.refreshing = false;
                } else if let PullsView::Pull(number) = this.view {
                    this.refreshing = false;
                    this.load_pull(number, cx);
                } else {
                    this.reload_list(cx);
                }
                cx.notify();
            },
            cx,
        );
        cx.notify();
    }

    /// The first page again (`more == false`), or the next one.
    pub fn load_list(&mut self, more: bool, cx: &mut Context<Self>) {
        let Some(target) = self.ready().cloned() else {
            return;
        };
        if self.loading {
            return;
        }
        let filter = PullFilter { state: self.state };
        let page = if more { self.page + 1 } else { 1 };
        self.loading = true;
        spawn_load(
            self,
            |s| &mut s.list_load,
            move || {
                let client = target.client()?;
                // A Closed or Merged filter on GitHub and Gitea drops the other from a page; read on until one is not empty.
                let mut page = page;
                loop {
                    let items = client.pulls(&target.repo, &filter, page)?;
                    let full = items.len() >= PAGE_SIZE || needs_more(&target, filter.state);
                    if !items.is_empty() || !full || page > 20 {
                        return Ok::<_, gd_forge::Error>((items, page, full));
                    }
                    page += 1;
                }
            },
            move |this, res, cx| {
                this.loading = false;
                this.refreshing = false;
                match res {
                    Ok((items, page, full)) => {
                        this.list_error = None;
                        this.page = page;
                        this.more = full && !items.is_empty();
                        if !more {
                            this.pulls.clear();
                        }
                        let known: HashSet<u64> = this.pulls.iter().map(|p| p.number).collect();
                        this.pulls
                            .extend(items.into_iter().filter(|p| !known.contains(&p.number)));
                    }
                    Err(e) => this.list_error = Some(e.into()),
                }
                cx.notify();
            },
            cx,
        );
        cx.notify();
    }

    pub fn reload_list(&mut self, cx: &mut Context<Self>) {
        self.list_load.cancel();
        self.loading = false;
        self.more = false;
        self.list_error = None;
        self.load_list(false, cx);
    }

    pub fn set_state(&mut self, state: Option<PullState>, cx: &mut Context<Self>) {
        self.state = state;
        self.pulls.clear();
        self.reload_list(cx);
    }

    pub fn open_pull(&mut self, number: u64, cx: &mut Context<Self>) {
        self.forward.clear();
        self.show_pull(number, cx);
    }

    fn show_pull(&mut self, number: u64, cx: &mut Context<Self>) {
        self.view = PullsView::Pull(number);
        self.tab = PullTab::Conversation;
        if self.detail.as_ref().is_none_or(|d| d.number != number) {
            self.detail = None;
            self.conversation = None;
            self.head = None;
        }
        self.detail_error = None;
        self.conversation_error = None;
        self.commit = None;
        self.pick = None;
        self.replying = None;
        self.load_pull(number, cx);
        cx.notify();
    }

    /// The detail, then its head fetched into the repository; the conversation alongside.
    fn load_pull(&mut self, number: u64, cx: &mut Context<Self>) {
        let Some(target) = self.ready().cloned() else {
            return;
        };
        let t = target.clone();
        spawn_load(
            self,
            |s| &mut s.detail_load,
            move || {
                let pull = t.client()?.pull(&t.repo, number)?;
                let head = fetch_head(&t, &pull);
                Ok::<_, gd_forge::Error>((pull, head))
            },
            |this, res, cx| {
                match res {
                    Ok((pull, head)) => {
                        this.detail = Some(pull);
                        this.head = Some(head);
                        this.show_tab_files(cx);
                    }
                    Err(e) => this.detail_error = Some(e.into()),
                }
                cx.notify();
            },
            cx,
        );
        self.load_conversation(number, target, cx);
    }

    fn load_conversation(&mut self, number: u64, target: Target, cx: &mut Context<Self>) {
        spawn_load(
            self,
            |s| &mut s.conversation_load,
            move || target.client()?.conversation(&target.repo, number),
            |this, res, cx| {
                match res {
                    Ok(c) => {
                        this.conversation_error = None;
                        this.conversation = Some(c);
                    }
                    Err(e) => this.conversation_error = Some(e.into()),
                }
                cx.notify();
            },
            cx,
        );
    }

    /// The Refresh button of a pull request: everything again.
    pub fn refresh_pull(&mut self, cx: &mut Context<Self>) {
        if let PullsView::Pull(number) = self.view {
            self.load_pull(number, cx);
        }
    }

    pub fn set_tab(&mut self, tab: PullTab, cx: &mut Context<Self>) {
        self.tab = tab;
        if tab == PullTab::Commits && self.commit.is_none() {
            self.commit = self
                .pull_head()
                .is_some_and(|h| !h.commits.is_empty())
                .then_some(0);
        }
        self.show_tab_files(cx);
        cx.notify();
    }

    pub fn select_commit(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.commit = Some(ix);
        self.show_tab_files(cx);
        cx.notify();
    }

    /// Ask the workspace to show the files of the tab: the whole pull request, or the selected commit.
    fn show_tab_files(&mut self, cx: &mut Context<Self>) {
        let Some(head) = self.pull_head() else {
            return;
        };
        let range = match (self.tab, self.commit) {
            (PullTab::Commits, Some(ix)) => head.commits.get(ix).map(|c| {
                let parent = c
                    .parents
                    .first()
                    .cloned()
                    .unwrap_or_else(|| format!("{}^", c.sha));
                (parent, c.sha.clone())
            }),
            (PullTab::Files, _) => Some((head.merge_base.clone(), head.head.clone())),
            _ => None,
        };
        if let Some((base, head)) = range {
            cx.emit(PullsEvent::Review { base, head });
        }
    }

    /// A line number of the review diff was clicked: pick that line, or (Shift) extend the pick to it within the file.
    pub fn pick_line(&mut self, path: String, ix: usize, extend: bool, cx: &mut Context<Self>) {
        self.pick = Some(match &self.pick {
            Some(p) if extend && p.path == path => {
                let (start, end) = if ix < p.start {
                    (ix, p.end)
                } else {
                    (p.start, ix)
                };
                Pick { path, start, end }
            }
            _ => Pick {
                path,
                start: ix,
                end: ix,
            },
        });
        self.replying = None;
        cx.notify();
    }

    pub fn cancel_pick(&mut self, cx: &mut Context<Self>) {
        self.pick = None;
        cx.notify();
    }

    pub fn start_reply(&mut self, thread: Option<String>, cx: &mut Context<Self>) {
        self.replying = thread;
        self.pick = None;
        cx.notify();
    }

    /// Send the text of the box that belongs to `on`; on success clear it and reload the conversation.
    pub fn post(&mut self, on: Reply, cx: &mut Context<Self>) {
        let (Some(target), Some(pull)) = (self.ready().cloned(), self.detail.clone()) else {
            return;
        };
        let input = match on {
            Reply::Conversation => self.inputs.comment.clone(),
            Reply::Lines(_) => self.inputs.line.clone(),
            Reply::Thread(_) => self.inputs.reply.clone(),
        };
        let body = input.read(cx).as_str().trim().to_string();
        if body.is_empty() || self.posting {
            return;
        }
        self.posting = true;
        let (epoch, number) = (self.epoch, pull.number);
        let t = target.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_spawn(async move {
                    let client = t.client()?;
                    match &on {
                        Reply::Conversation => client.comment(&t.repo, pull.number, &body),
                        Reply::Lines(line) => client.line_comment(&t.repo, &pull, line, &body),
                        Reply::Thread(thread) => client.reply(&t.repo, &pull, thread, &body),
                    }
                })
                .await;
            this.update(cx, |this, cx| {
                if this.epoch != epoch {
                    return;
                }
                this.posting = false;
                match res {
                    Ok(()) => {
                        input.update(cx, |s, cx| s.emplace("", cx));
                        this.pick = None;
                        this.replying = None;
                        this.load_conversation(number, target, cx);
                        cx.emit(PullsEvent::Notice("Comment posted".into()));
                    }
                    Err(e) => cx.emit(PullsEvent::Error(format!("Couldn't post the comment: {e}"))),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Check the open pull request out: its head branch tracking the remote one, or `pr/<n>` for a fork.
    pub fn checkout(&mut self, cx: &mut Context<Self>) {
        let (Some(target), Some(pull)) = (self.ready(), &self.detail) else {
            return;
        };
        let pull_ref = pull_ref(target.forge == Forge::GitLab, pull.number);
        let branch = match (&pull.head_ref, pull.from_fork) {
            (Some(head), false) => head.clone(),
            _ => format!("pr/{}", pull.number),
        };
        let upstream = pull.head_ref.clone().filter(|_| !pull.from_fork);
        cx.emit(PullsEvent::Checkout(PullCheckout {
            pull_ref,
            branch,
            upstream,
        }));
    }
}

/// GitHub and Gitea have no Merged filter: the Closed and Merged lists are filtered from all closed pull requests.
fn needs_more(target: &Target, state: Option<PullState>) -> bool {
    target.forge != Forge::GitLab && matches!(state, Some(PullState::Closed | PullState::Merged))
}

/// Fetch the head of `pull` (and its branch when it is in this repository) and compare it with the base. Blocks on git.
fn fetch_head(target: &Target, pull: &PullRequest) -> Result<gd_git::PullHead, String> {
    let repo = Repository::open(&target.root).map_err(|e| e.to_string())?;
    let base = pull
        .base_ref
        .as_deref()
        .ok_or("The pull request has no base branch.")?;
    let head_branch = pull.head_ref.as_deref().filter(|_| !pull.from_fork);
    repo.fetch_pull(
        &pull_ref(target.forge == Forge::GitLab, pull.number),
        base,
        head_branch,
    )
    .map_err(|e| e.to_string())
}
