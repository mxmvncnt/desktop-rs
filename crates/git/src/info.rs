//! Facts about a repository for the repository settings page.

use crate::{Repository, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoInfo {
    pub head: Option<String>,
    /// `origin`, else the first remote.
    pub default_remote: Option<String>,
    /// Loose objects plus packs, in KiB.
    pub size_kib: u64,
}

impl Repository {
    pub fn info(&self) -> Result<RepoInfo> {
        let head = self.git().args(["rev-parse", "--verify", "-q", "HEAD"]).run().ok().map(|s| s.trim().to_string());
        let remotes = self.remotes()?;
        let default_remote = crate::default_remote(&remotes).map(str::to_string);
        // `size: <KiB>` and `size-pack: <KiB>` lines.
        let counts = self.git().args(["count-objects", "-v"]).run()?;
        let kib = |key: &str| counts.lines().find_map(|l| l.strip_prefix(key)?.trim().parse::<u64>().ok()).unwrap_or(0);
        Ok(RepoInfo { head, default_remote, size_kib: kib("size:") + kib("size-pack:") })
    }
}
