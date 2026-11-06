//! `gd-forge`: CI pipelines and pull requests of GitHub, GitLab, Gitea and Forgejo behind one blocking client.
//! Pipelines are normalized to GitLab's vocabulary, pull requests to GitHub's (`docs/ARCHITECTURE.md` §Forge crate).
//!
//! No auth flow, no capability probing: the caller knows the platform, host and token, and the
//! platform fixes what is possible (`Client::capabilities`). Every call blocks: run it on a background executor.

mod actions;
mod client;
mod error;
mod forgejo;
mod gitea_pulls;
mod github_pulls;
mod gitlab;
mod gitlab_pulls;
mod log;
mod model;
mod pull_model;
mod pull_query;
mod repo_ref;
mod workflow;

pub use client::{Client, Forge};
pub use error::{Error, Result};
pub use log::{Color, Line, Log, Section, Span, Style, parse_log};
pub use model::*;
pub use pull_model::*;
pub use pull_query::{Expr, PullQuery, SortKey};
pub use repo_ref::RepoRef;
pub use workflow::stages_from_workflow;
