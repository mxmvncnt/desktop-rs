//! `RepoModel`: all state for the currently-open repository, plus the async
//! plumbing that runs git on the background executor.
//!
//! Rules (see docs/ARCHITECTURE.md §Threading):
//! - Never call `gd_git` on the UI thread. Use [`RepoModel::run`] /
//!   `cx.background_spawn` and apply results in `this.update(..)`.
//! - Stale results are dropped: [`RepoModel::load`] / [`spawn_load`] keep a
//!   generation in a [`LoadSlot`] and ignore results from an older one (the user
//!   may have clicked another file while a big diff was loading).
//!
//! Files: `state.rs` (struct + events), `refresh.rs` (coalesced refresh,
//! watcher), `background_fetch.rs`, `change_diff.rs` / `commit_diff.rs`
//! (diff loading), `diff_highlight.rs`, `expand.rs`,
//! `history_log.rs` (paged log), `run.rs` (write-op runner, `LoadSlot` loads).
//! Feature-specific ops live in `features/<x>/ops.rs`.

mod background_fetch;
mod change_diff;
mod commit_diff;
mod diff_highlight;
mod expand;
mod history_log;
mod refresh;
mod run;
mod state;

pub use run::{LoadSlot, Op, spawn_load};
pub use state::{LastCommit, RemoteOp, RepoEvent, RepoModel, Tab};
