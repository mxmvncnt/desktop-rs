//! `gd-forge`: CI pipelines of GitHub, GitLab, Gitea and Forgejo behind one blocking client,
//! normalized to GitLab's vocabulary (`docs/ARCHITECTURE.md` §Forge crate).
//!
//! No auth flow, no capability probing: the caller knows the platform, host and token, and the
//! platform fixes what is possible (`Client::capabilities`). Every call blocks: run it on a background executor.

mod actions;
mod client;
mod error;
mod forgejo;
mod gitlab;
mod log;
mod model;
mod repo_ref;
mod workflow;

pub use client::{Client, Forge};
pub use log::{Color, Line, Log, Section, Span, Style, parse_log};
pub use error::{Error, Result};
pub use model::*;
pub use repo_ref::RepoRef;
pub use workflow::stages_from_workflow;
