//! Remotes as listed and edited by `git remote`.

use crate::{Repository, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    pub name: String,
    pub fetch_url: String,
    /// The push URL; equal to the fetch URL unless `remote.<name>.pushurl` is set.
    pub push_url: String,
}

impl Remote {
    pub fn has_push_url(&self) -> bool {
        self.push_url != self.fetch_url
    }
}

impl Repository {
    /// Every remote with its fetch and push URL, from `git remote -v` (`<name>\t<url> (fetch|push)`).
    pub fn remote_list(&self) -> Result<Vec<Remote>> {
        let out = self.git().args(["remote", "-v"]).run()?;
        let mut list: Vec<Remote> = Vec::new();
        for line in out.lines() {
            let Some((name, rest)) = line.split_once('\t') else {
                continue;
            };
            let (url, kind) = rest.rsplit_once(' ').unwrap_or((rest, ""));
            match list.iter_mut().find(|r| r.name == name) {
                Some(r) if kind == "(push)" => r.push_url = url.into(),
                Some(_) => {}
                None => list.push(Remote {
                    name: name.into(),
                    fetch_url: url.into(),
                    push_url: url.into(),
                }),
            }
        }
        Ok(list)
    }

    pub fn add_remote(&self, name: &str, url: &str) -> Result<()> {
        self.git().args(["remote", "add", "--", name, url]).exec()
    }

    pub fn rename_remote(&self, old: &str, new: &str) -> Result<()> {
        self.git().args(["remote", "rename", old, new]).exec()
    }

    pub fn remove_remote(&self, name: &str) -> Result<()> {
        self.git().args(["remote", "remove", name]).exec()
    }

    pub fn set_remote_url(&self, name: &str, url: &str) -> Result<()> {
        self.git().args(["remote", "set-url", name, url]).exec()
    }

    /// Set the push URL, or drop it (push to the fetch URL again) when `url` is empty.
    pub fn set_remote_push_url(&self, name: &str, url: &str) -> Result<()> {
        if url.is_empty() {
            // Exit 5 when no push URL is set: nothing to drop.
            let _ = self
                .git()
                .args([
                    "config",
                    "--local",
                    "--unset-all",
                    &format!("remote.{name}.pushurl"),
                ])
                .exec();
            return Ok(());
        }
        self.git()
            .args(["remote", "set-url", "--push", name, url])
            .exec()
    }
}
