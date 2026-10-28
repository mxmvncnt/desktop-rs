//! The write-op runner every feature's `ops.rs` builds on.

use gd_git::Repository;
use gpui::{AppContext, Context, Task};

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

/// Generation counter plus the task of the newest load: starting a load drops
/// the previous task, and a result from an older generation is never applied.
#[derive(Default)]
pub struct LoadSlot {
    generation: u64,
    /// Newest generation applied (`late` slots only).
    applied: u64,
    late: bool,
    task: Option<Task<()>>,
}

impl LoadSlot {
    /// A slot whose loads keep running when a newer one starts: a late result
    /// still applies unless a newer one already did. Holding an arrow key then
    /// shows each commit as it lands instead of only the last one.
    pub fn late() -> Self {
        Self { late: true, ..Self::default() }
    }

    /// Drop the load in flight (and its result).
    pub fn cancel(&mut self) {
        self.generation += 1;
        self.task = None;
    }
}

/// Run `work` off-thread and hand its result to `apply` on the UI thread,
/// unless a newer load of the same `slot` started meanwhile.
pub fn spawn_load<E: 'static, T: Send + 'static>(
    this: &mut E,
    slot: impl Fn(&mut E) -> &mut LoadSlot + Clone + 'static,
    work: impl FnOnce() -> T + Send + 'static,
    apply: impl FnOnce(&mut E, T, &mut Context<E>) + 'static,
    cx: &mut Context<E>,
) {
    let slot_ref = slot(this);
    slot_ref.cancel();
    let generation = slot_ref.generation;
    let late = slot_ref.late;
    let slot_in_task = slot.clone();
    let task = cx.spawn(async move |this, cx| {
        let res = cx.background_spawn(async move { work() }).await;
        this.update(cx, |this, cx| {
            let slot = slot_in_task(this);
            let fresh = if slot.late { generation >= slot.applied } else { generation == slot.generation };
            if fresh {
                slot.applied = generation;
                apply(this, res, cx);
            }
        })
        .ok();
    });
    if late {
        task.detach();
    } else {
        slot(this).task = Some(task);
    }
}

impl RepoModel {
    /// [`spawn_load`] for a git read: errors go to `self.error`, `apply` only sees successes.
    pub fn load<T: Send + 'static>(
        &mut self,
        slot: impl Fn(&mut Self) -> &mut LoadSlot + Clone + 'static,
        work: impl FnOnce(&Repository) -> gd_git::Result<T> + Send + 'static,
        apply: impl FnOnce(&mut Self, T, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        let repo = self.repo.clone();
        spawn_load(self, slot, move || work(&repo), |this, res, cx| {
            match res {
                Ok(v) => apply(this, v, cx),
                Err(e) => this.error = Some(e.to_string()),
            }
            cx.notify();
        }, cx);
    }
}
