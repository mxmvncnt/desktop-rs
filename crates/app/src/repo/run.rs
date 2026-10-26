//! The write-op runner every feature's `ops.rs` builds on.

use gd_git::Repository;
use gpui::{AppContext, Context};

use super::state::{RemoteOp, RepoModel};

/// What an op marks as in flight while it runs. `run` clears exactly what it set.
#[derive(Default)]
pub struct Op {
    /// Merge `busy` label: shown in the banner, and only one labeled op runs at a time.
    pub label: Option<&'static str>,
    pub committing: bool,
    pub remote: Option<RemoteOp>,
}

impl Op {
    pub fn labeled(label: &'static str) -> Self {
        Self { label: Some(label), ..Self::default() }
    }
}

impl RepoModel {
    /// Run a blocking git operation off-thread, then refresh. `on_ok` runs on
    /// the UI thread after success; errors go to `self.error`.
    pub fn run<R: Send + 'static>(
        &mut self,
        op: Op,
        work: impl FnOnce(&Repository) -> gd_git::Result<R> + Send + 'static,
        on_ok: impl FnOnce(&mut Self, R, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        if let Some(label) = op.label {
            if self.merge.busy.is_some() {
                return;
            }
            self.merge.busy = Some(label);
            self.history.multi.clear();
        }
        self.committing |= op.committing;
        self.remote_op = op.remote.or(self.remote_op);
        self.error = None;
        let repo = self.repo.clone();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let res = cx.background_spawn(async move { work(&repo) }).await;
            this.update(cx, |this, cx| {
                if op.label.is_some() {
                    this.merge.busy = None;
                }
                this.committing &= !op.committing;
                if op.remote.is_some() {
                    this.remote_op = None;
                }
                match res {
                    Ok(r) => on_ok(this, r, cx),
                    Err(e) => this.error = Some(e.to_string()),
                }
                this.refresh(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
