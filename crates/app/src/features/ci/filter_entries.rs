//! What the dropdowns of the branch and user filters list.

use std::collections::HashSet;

use gd_forge::User;
use gd_git::{Branch, BranchKind};

use crate::components::ComboEntry;

/// The branches a pipeline can be filtered on that contain `typed` (ignoring case): the local ones, then the remote ones that are not local
/// (without their remote), the current branch first.
pub fn branch_entries(branches: &[Branch], current: Option<&str>, typed: &str) -> Vec<ComboEntry> {
    let typed = typed.trim().to_lowercase();
    let mut seen = HashSet::new();
    let mut names: Vec<String> = branches
        .iter()
        .filter_map(|b| match b.kind {
            BranchKind::Local => Some(b.name.clone()),
            BranchKind::Remote => b.name.split_once('/').map(|(_, name)| name.to_string()),
        })
        .filter(|name| name != "HEAD" && seen.insert(name.clone()))
        .filter(|name| name.to_lowercase().contains(&typed))
        .collect();
    if let Some(ix) = names.iter().position(|n| Some(n.as_str()) == current) {
        names[..=ix].rotate_right(1);
    }
    names
        .into_iter()
        .map(|name| ComboEntry {
            detail: (Some(name.as_str()) == current).then(|| "current".to_string()),
            value: name,
        })
        .collect()
}

/// The users that contain `typed` (ignoring case) in their login or name, those that start with it first; all of them when nothing was typed.
/// Each login once.
pub fn user_entries<'a>(users: impl IntoIterator<Item = &'a User>, typed: &str) -> Vec<ComboEntry> {
    let typed = typed.trim().to_lowercase();
    let mut seen = HashSet::new();
    let mut found: Vec<(bool, ComboEntry)> = users
        .into_iter()
        .filter(|u| seen.insert(u.username.to_lowercase()))
        .filter_map(|u| {
            let (login, name) = (
                u.username.to_lowercase(),
                u.name.as_deref().unwrap_or("").to_lowercase(),
            );
            (login.contains(&typed) || name.contains(&typed)).then(|| {
                (
                    login.starts_with(&typed) || name.starts_with(&typed),
                    ComboEntry {
                        value: u.username.clone(),
                        detail: u.name.clone(),
                    },
                )
            })
        })
        .collect();
    found.sort_by_key(|(prefix, e)| (!prefix, e.value.to_lowercase()));
    found.into_iter().map(|(_, e)| e).collect()
}

#[cfg(test)]
#[path = "tests/filter_entries.test.rs"]
mod tests;
