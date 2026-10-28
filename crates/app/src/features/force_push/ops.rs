//! Force push preview operations on `RepoModel`. The load runs on the
//! background executor; the detail's [`LoadSlot`](crate::repo::LoadSlot) drops stale results.

use gpui::Context;

use crate::features::diff::DiffPane;
use crate::repo::{RepoModel, spawn_load};

impl RepoModel {
    pub fn load_force_push(&mut self, cx: &mut Context<Self>) {
        self.force_push.loading = true;
        self.force_push.error = None;
        let (repo, opts) = (self.repo.clone(), self.diff_options);
        spawn_load(
            self,
            |this| &mut this.force_push.detail.load,
            move || {
                let preview = repo.force_push_preview()?;
                let first = match preview.files.first() {
                    Some(f) => Some((DiffPane::ForcePush.file_diff(&repo, &preview.remote_tip, f, opts)?, f.path.clone())),
                    None => None,
                };
                Ok::<_, gd_git::GitError>((preview, first))
            },
            |this, res, cx| {
                this.force_push.loading = false;
                match res {
                    Ok((p, first)) => {
                        let (tip, files, stats) = (p.remote_tip.clone(), p.files.clone(), p.stats);
                        this.force_push.preview = Some(p);
                        this.show_files(DiffPane::ForcePush, tip, files, stats, first, cx);
                    }
                    Err(e) => {
                        this.force_push.preview = None;
                        this.force_push.error = Some(e.to_string());
                        this.force_push.detail.clear();
                    }
                }
                cx.notify();
            },
            cx,
        );
        cx.notify();
    }
}
