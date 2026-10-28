//! Workspace-side picker handlers: build the popover components from the
//! workspace state, keyboard cursor, and the actions the popovers trigger.

use gpui::{Context, Window};

use super::branch_picker::BranchPicker;
use super::clone_form::CloneForm;
use super::new_branch_base::NewBranchBase;
use super::new_branch_form::NewBranchForm;
use super::repo_picker::{RepoEntry, RepoPicker};
use crate::utils::text::sanitize_branch_name;
use crate::workspace::{Popover, Workspace};

impl Workspace {
    pub fn repo_picker(&mut self, cx: &mut Context<Self>) -> RepoPicker {
        let items = self.filtered_repos(cx);
        let (selected, filter, scroll) = self.pickers.update(cx, |p, _| (p.clamp(items.len()), p.filter.clone(), p.scroll.clone()));
        let entries = items
            .into_iter()
            .filter_map(|ix| {
                let path = self.settings.repositories.get(ix)?;
                Some(RepoEntry {
                    ix,
                    name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                    current: self.settings.selected == Some(ix),
                })
            })
            .collect();
        RepoPicker {
            entries,
            selected,
            filter,
            scroll,
            pickers: self.pickers.clone(),
            ws: cx.entity(),
        }
    }

    /// `None` when no repository is open.
    pub fn branch_picker(&mut self, cx: &mut Context<Self>) -> Option<BranchPicker> {
        let repo = self.repo.clone()?;
        let items = self.filtered_branches(cx);
        let (selected, filter, scroll) = self.pickers.update(cx, |p, _| (p.clamp(items.len()), p.filter.clone(), p.scroll.clone()));
        Some(BranchPicker {
            items,
            selected,
            repo,
            filter,
            scroll,
            pickers: self.pickers.clone(),
            ws: cx.entity(),
        })
    }

    pub fn new_branch_form(&self, cx: &mut Context<Self>) -> NewBranchForm {
        let repo = self.repo.as_ref().map(|r| r.read(cx));
        let p = self.pickers.read(cx);
        NewBranchForm {
            name: sanitize_branch_name(p.name_input.read(cx).as_str()),
            base: NewBranchBase {
                current: repo.map(|r| r.branch_name()).unwrap_or_default(),
                chosen: p.new_branch_base.clone(),
                open: p.new_branch_base_open,
                branches: repo.map(|r| r.branches.clone()).unwrap_or_default(),
                pickers: self.pickers.clone(),
            },
            input: p.name_input.clone(),
            ws: cx.entity(),
        }
    }

    pub fn clone_form(&self, cx: &mut Context<Self>) -> CloneForm {
        let url_input = &self.pickers.read(cx).url_input;
        CloneForm {
            url_empty: url_input.read(cx).as_str().trim().is_empty(),
            input: url_input.clone(),
            ws: cx.entity(),
        }
    }

    /// Enter in the repository picker: open the highlighted repository.
    pub(super) fn enter_repo_picker(&mut self, cx: &mut Context<Self>) {
        if let Some(&ix) = self.filtered_repos(cx).get(self.pickers.read(cx).selected) {
            self.open_repository(ix, cx)
        }
    }

    /// Enter in the branch picker: check out the highlighted branch.
    pub(super) fn enter_branch_picker(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else { return };
        let Some(&ix) = self.filtered_branches(cx).get(self.pickers.read(cx).selected) else { return };
        let b = repo.read(cx).branches[ix].clone();
        if !b.is_head {
            repo.update(cx, |r, cx| r.checkout(b, cx));
        }
        self.close_overlay(w, cx);
    }

    /// Click on a branch row.
    pub(super) fn pick_branch(&mut self, b: gd_git::Branch, w: &mut Window, cx: &mut Context<Self>) {
        if let (false, Some(repo)) = (b.is_head, &self.repo) {
            repo.update(cx, |r, cx| r.checkout(b, cx));
        }
        self.close_overlay(w, cx);
    }

    /// "New branch" footer button: the filter text prefills the name.
    pub(super) fn open_new_branch_from_filter(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let prefill = self.pickers.read(cx).filter.read(cx).as_str().to_string();
        self.toggle_popover(Popover::NewBranch, w, cx);
        self.pickers.read(cx).name_input.clone().update(cx, |s, cx| s.emplace(&prefill, cx));
    }

    pub(super) fn submit_new_branch(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let p = self.pickers.read(cx);
        let name = sanitize_branch_name(p.name_input.read(cx).as_str());
        if name.is_empty() {
            return;
        }
        if let Some(repo) = &self.repo {
            let base = self.pickers.update(cx, |p, _| p.new_branch_base.take());
            match base {
                Some(base) => repo.update(cx, |r, cx| r.create_branch_from(name, base, cx)),
                None => repo.update(cx, |r, cx| r.create_branch(name, cx)),
            }
        }
        self.close_overlay(w, cx);
    }

    pub(super) fn submit_clone(&mut self, cx: &mut Context<Self>) {
        let url = self.pickers.read(cx).url_input.read(cx).as_str().to_string();
        self.clone_repository(url, cx)
    }
}
