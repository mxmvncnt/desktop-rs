//! `impl Workspace`: adding, opening, removing and cloning repositories.

use std::path::PathBuf;

use gd_git::Repository;
use gpui::{AppContext, Context, PathPromptOptions};

use super::state::Workspace;
use crate::features::notices::observe_notices;
use crate::repo::{RepoEvent, RepoModel};

impl Workspace {
    pub fn add_repository(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        match Repository::open(&path) {
            Ok(repo) => {
                let root = repo.root().to_path_buf();
                let ix = match self.settings.repositories.iter().position(|p| *p == root) {
                    Some(ix) => ix,
                    None => {
                        self.settings.repositories.push(root);
                        self.settings.repositories.len() - 1
                    }
                };
                self.open_repository(ix, cx);
            }
            Err(e) => self.show_error(e.to_string(), cx),
        }
    }

    pub fn open_repository(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(path) = self.settings.repositories.get(ix).cloned() else {
            return;
        };
        self.settings.selected = Some(ix);
        self.settings.save();
        self.overlays.clear();
        match Repository::open(&path) {
            Ok(repo) => self.attach_model(repo, cx),
            Err(e) => {
                self.repo = None;
                self.show_error(e.to_string(), cx);
            }
        }
        cx.notify();
    }

    /// Build the `RepoModel` for `repo`, wire its observers, make it current.
    fn attach_model(&mut self, repo: Repository, cx: &mut Context<Self>) {
        let model = cx.new(|cx| RepoModel::new(repo, cx));
        model.update(cx, |m, _| {
            m.diff_options.ignore_whitespace = self.settings.diff_ignore_whitespace
        });
        self.repo_sub = Some(cx.observe(&model, observe_notices));
        self.commit_box.update(cx, |b, cx| b.attach(&model, cx));
        self.repo_event_sub = Some(cx.subscribe(&model, |this, _, ev: &RepoEvent, cx| this.on_repo_event(ev, cx)));
        self.repo = Some(model);
        self.overlays.clear();
    }

    fn on_repo_event(&mut self, ev: &RepoEvent, cx: &mut Context<Self>) {
        match ev {
            RepoEvent::CommitUndone { summary, description } => self.commit_box.update(cx, |b, cx| b.set_text(summary, description, cx)),
            RepoEvent::SwitchBranchWithChanges(branch) => self.open_switch_dialog(branch.clone(), cx),
            RepoEvent::PreparedMessage { summary, description } => {
                // A merge paused: show git's prepared message unless the user typed one.
                self.commit_box.update(cx, |b, cx| {
                    if b.message.read(cx).as_str().trim().is_empty() {
                        b.set_text(summary, description, cx);
                    }
                });
            }
        }
    }

    pub fn remove_repository(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix >= self.settings.repositories.len() {
            return;
        }
        self.settings.repositories.remove(ix);
        match self.settings.selected {
            Some(s) if s == ix => {
                self.settings.selected = None;
                self.repo = None;
            }
            Some(s) if s > ix => self.settings.selected = Some(s - 1),
            _ => {}
        }
        self.settings.save();
        cx.notify();
    }

    pub fn prompt_add_repository(&mut self, cx: &mut Context<Self>) {
        self.overlays.clear();
        let paths = cx.prompt_for_paths(directory_prompt("Add repository"));
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await
                && let Some(path) = paths.into_iter().next()
            {
                this.update(cx, |this, cx| this.add_repository(path, cx)).ok();
            }
        })
        .detach();
    }

    pub fn clone_repository(&mut self, url: String, cx: &mut Context<Self>) {
        let url = url.trim().to_string();
        if url.is_empty() {
            return;
        }
        let home = dirs::home_dir().unwrap_or_default();
        let dest = cx.prompt_for_new_path(&home, Some(&clone_dir_name(&url)));
        self.overlays.clear();
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(dest))) = dest.await else { return };
            let name = dest.file_name().unwrap_or_default().to_string_lossy().into_owned();
            this.update(cx, |this, cx| this.show_notice(format!("Cloning {name}…"), cx))
            .ok();
            let res = cx
                .background_spawn({
                    let dest = dest.clone();
                    async move { Repository::clone(&url, &dest) }
                })
                .await;
            this.update(cx, |this, cx| match res {
                Ok(repo) => this.add_repository(repo.root().to_path_buf(), cx),
                Err(e) => this.show_error(format!("Clone failed: {e}"), cx),
            })
            .ok();
        })
        .detach();
    }
}

fn directory_prompt(prompt: &'static str) -> PathPromptOptions {
    PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some(prompt.into()) }
}

/// Folder name `git clone` would pick: last path segment without `.git`.
fn clone_dir_name(url: &str) -> String {
    url.trim_end_matches('/')
        .rsplit(['/', ':'])
        .next()
        .unwrap_or("repository")
        .trim_end_matches(".git")
        .to_string()
}
