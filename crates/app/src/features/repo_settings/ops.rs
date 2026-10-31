//! `impl RepoSettingsStore`: background reads and writes of the open repository.

use gd_git::Repository;
use gpui::{AppContext as _, Context, Entity};
use gpui_elements::editable_text::EditableTextState;

use super::state::{Edit, RepoSettingsStore};
use crate::repo::{RepoModel, spawn_load};

fn text(input: &Entity<EditableTextState>, cx: &Context<RepoSettingsStore>) -> String {
    input.read(cx).as_str().trim().to_string()
}

fn set_text(input: &Entity<EditableTextState>, text: &str, cx: &mut Context<RepoSettingsStore>) {
    input.update(cx, |s, cx| s.emplace(text, cx));
}

impl RepoSettingsStore {
    /// Reset to the first page and read the repository off the UI thread.
    pub fn open(&mut self, model: Entity<RepoModel>, cx: &mut Context<Self>) {
        self.page = Default::default();
        (self.edit, self.error) = (None, None);
        self.model = Some(model);
        self.reload(cx);
    }

    fn repo(&self, cx: &Context<Self>) -> Option<Repository> {
        Some(self.model.as_ref()?.read(cx).repo.clone())
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        let Some(repo) = self.repo(cx) else { return };
        spawn_load(
            self,
            |this| &mut this.load,
            move || Ok::<_, gd_git::GitError>((repo.info()?, repo.remote_list()?)),
            |this, res, cx| {
                match res {
                    Ok((info, remotes)) => (this.info, this.remotes) = (Some(info), remotes),
                    Err(e) => this.error = Some(e.to_string()),
                }
                cx.notify();
            },
            cx,
        );
    }

    /// Run a git write off the UI thread. On success the form closes, the page reloads and the
    /// toolbar refreshes (remotes change the push/pull button); on failure the error stays on the page.
    fn write(&mut self, work: impl FnOnce(&Repository) -> gd_git::Result<()> + Send + 'static, cx: &mut Context<Self>) {
        let Some(repo) = self.repo(cx) else { return };
        self.error = None;
        cx.spawn(async move |this, cx| {
            let res = cx.background_spawn(async move { work(&repo) }).await;
            this.update(cx, |this, cx| {
                match res {
                    Ok(()) => {
                        this.edit = None;
                        this.reload(cx);
                        if let Some(model) = this.model.clone() {
                            model.update(cx, |m, cx| m.refresh(cx));
                        }
                    }
                    Err(e) => this.error = Some(e.to_string()),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub fn close_edit(&mut self, cx: &mut Context<Self>) {
        (self.edit, self.error) = (None, None);
        cx.notify();
    }

    // ---- remotes -----------------------------------------------------------

    /// Open the form to add a remote, or to edit the remote called `editing`.
    pub fn open_remote_form(&mut self, editing: Option<String>, cx: &mut Context<Self>) {
        let remote = editing.as_ref().and_then(|n| self.remotes.iter().find(|r| &r.name == n)).cloned();
        let inp = self.inputs.clone();
        let push = remote.as_ref().filter(|r| r.has_push_url()).map(|r| r.push_url.clone());
        set_text(&inp.remote_name, remote.as_ref().map_or("", |r| &r.name), cx);
        set_text(&inp.remote_url, remote.as_ref().map_or("", |r| &r.fetch_url), cx);
        set_text(&inp.remote_push_url, push.as_deref().unwrap_or(""), cx);
        self.edit = Some(editing.map_or(Edit::AddRemote, Edit::Remote));
        self.error = None;
        cx.notify();
    }

    pub fn save_remote(&mut self, cx: &mut Context<Self>) {
        let inp = self.inputs.clone();
        let (name, url, push) = (text(&inp.remote_name, cx), text(&inp.remote_url, cx), text(&inp.remote_push_url, cx));
        if name.is_empty() || url.is_empty() {
            self.error = Some("A remote needs a name and a URL.".into());
            return cx.notify();
        }
        let old = match &self.edit {
            Some(Edit::Remote(old)) => self.remotes.iter().find(|r| &r.name == old).cloned(),
            _ => None,
        };
        self.write(
            move |repo| {
                match &old {
                    None => repo.add_remote(&name, &url)?,
                    Some(old) => {
                        if old.name != name {
                            repo.rename_remote(&old.name, &name)?;
                        }
                        if old.fetch_url != url {
                            repo.set_remote_url(&name, &url)?;
                        }
                    }
                }
                repo.set_remote_push_url(&name, &push)
            },
            cx,
        );
    }

    pub fn confirm_remove_remote(&mut self, name: String, cx: &mut Context<Self>) {
        self.edit = Some(Edit::RemoveRemote(name));
        self.error = None;
        cx.notify();
    }

    pub fn remove_remote(&mut self, name: String, cx: &mut Context<Self>) {
        self.write(move |repo| repo.remove_remote(&name), cx);
    }
}
