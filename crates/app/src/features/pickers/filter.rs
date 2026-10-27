//! Filtering of the repository and branch lists by the popover's filter input.

use std::collections::HashSet;

use gd_git::{Branch, BranchKind};

use crate::repo::RepoModel;
use crate::workspace::Workspace;

impl Workspace {
    fn filter_text(&self, cx: &gpui::App) -> String {
        self.filter.read(cx).as_str().trim().to_lowercase()
    }

    /// Indices into `settings.repositories` matching the filter.
    pub(super) fn filtered_repos(&self, cx: &gpui::App) -> Vec<usize> {
        let f = self.filter_text(cx);
        self.settings
            .repositories
            .iter()
            .enumerate()
            .filter(|(_, p)| f.is_empty() || p.display().to_string().to_lowercase().contains(&f))
            .map(|(i, _)| i)
            .collect()
    }

    /// Indices into `repo.branches` matching the filter.
    pub(super) fn filtered_branches(&self, cx: &gpui::App) -> Vec<usize> {
        let Some(repo) = &self.repo else { return Vec::new() };
        let filter = self.filter_text(cx);
        repo.read(cx).matching_branches(&filter).map(|(i, _)| i).collect()
    }
}

impl RepoModel {
    /// Branches (with their index) whose name contains `filter` (lowercase),
    /// minus remote branches that already have a local counterpart.
    pub(crate) fn matching_branches<'a>(&'a self, filter: &'a str) -> impl Iterator<Item = (usize, &'a Branch)> {
        let local: HashSet<&str> =
            self.branches.iter().filter(|b| b.kind == BranchKind::Local).map(|b| b.name.as_str()).collect();
        self.branches
            .iter()
            .enumerate()
            .filter(move |(_, b)| filter.is_empty() || b.name.to_lowercase().contains(filter))
            .filter(move |(_, b)| {
                b.kind == BranchKind::Local || !b.name.split_once('/').is_some_and(|(_, n)| local.contains(n))
            })
    }
}
