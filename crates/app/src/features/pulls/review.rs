//! `impl RepoModel`: the files and diffs of the review pane (`DiffPane::Review`), and checking a pull request out.

use gd_git::PullCheckout;
use gpui::Context;

use crate::features::diff::DiffPane;
use crate::repo::{Op, RepoModel};

impl RepoModel {
    /// List the files of `base..head` (a pull request against its merge base, or one commit) and show the first one.
    pub fn show_review(&mut self, base: String, head: String, cx: &mut Context<Self>) {
        let detail = &self.review;
        if detail.base.as_deref() == Some(base.as_str())
            && detail.head.as_deref() == Some(head.as_str())
        {
            return;
        }
        let loaded = head.clone();
        self.load(
            |this| &mut this.review.load,
            move |repo| repo.pull_head(&head, &base),
            move |this, pulled, cx| {
                this.review.head = Some(loaded);
                this.show_files(
                    DiffPane::Review,
                    pulled.merge_base,
                    pulled.files,
                    pulled.stats,
                    None,
                    cx,
                );
                this.select_file(DiffPane::Review, 0, cx);
            },
            cx,
        );
    }

    pub fn checkout_pull(&mut self, checkout: PullCheckout, cx: &mut Context<Self>) {
        self.run(
            Op::labeled("Checking out the pull request"),
            move |r| r.checkout_pull(&checkout),
            |this, _, _| {
                this.reset_history_selection();
                this.last_commit = None;
            },
            cx,
        );
    }
}
