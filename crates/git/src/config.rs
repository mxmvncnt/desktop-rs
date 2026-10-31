//! The repository-local git config (`.git/config`) as a list of key/value entries.
//! A multi-valued key is several entries; edits match an entry by key and value.

use crate::{GitError, Repository, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigEntry {
    /// `section.name` or `section.subsection.name`, section and name in lowercase as git lists them.
    pub key: String,
    pub value: String,
}

/// Parse `git config --list -z`: `key\nvalue\0` records (`key\0` when the value is absent).
fn parse_list(out: &str) -> Vec<ConfigEntry> {
    out.split('\0')
        .filter(|r| !r.is_empty())
        .map(|r| {
            let (key, value) = r.split_once('\n').unwrap_or((r, ""));
            ConfigEntry { key: key.into(), value: value.into() }
        })
        .collect()
}

/// `section.name` or `section.subsection.name`: the section and the name are letters, digits and `-`
/// and start with a letter; the subsection may hold anything but a newline.
pub fn validate_key(key: &str) -> Result<()> {
    let bad = |m: &str| Err(GitError::Invalid(format!("{m}: use section.key or section.subsection.key")));
    let Some((section, rest)) = key.split_once('.') else { return bad("Invalid key") };
    let name = rest.rsplit_once('.').map_or(rest, |(_, name)| name);
    let word = |w: &str| w.starts_with(|c: char| c.is_ascii_alphabetic()) && w.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !word(section) || !word(name) {
        return bad("Invalid key");
    }
    if key.contains(['\n', '\0']) {
        return bad("Invalid key");
    }
    Ok(())
}

impl Repository {
    pub fn local_config(&self) -> Result<Vec<ConfigEntry>> {
        Ok(parse_list(&self.git().args(["config", "--local", "--list", "-z"]).run()?))
    }

    /// Add `key = value`, keeping the other values of a multi-valued key.
    pub fn add_config(&self, key: &str, value: &str) -> Result<()> {
        validate_key(key)?;
        self.git().args(["config", "--local", "--add", "--", key, value]).exec()
    }

    /// Replace the entry `key = old` by `key = new`.
    pub fn replace_config(&self, key: &str, old: &str, new: &str) -> Result<()> {
        self.git().args(["config", "--local", "--fixed-value", "--replace-all", "--", key, new, old]).exec()
    }

    /// Remove the entry `key = value` only.
    pub fn remove_config(&self, key: &str, value: &str) -> Result<()> {
        self.git().args(["config", "--local", "--fixed-value", "--unset-all", "--", key, value]).exec()
    }
}

#[cfg(test)]
#[path = "tests/config.test.rs"]
mod tests;
