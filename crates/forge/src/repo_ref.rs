use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'_').remove(b'.');

/// A repository on a forge host.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepoRef {
    /// Lowercase host of the remote (`github.com`).
    pub host: String,
    /// Owner, or the whole namespace for GitLab subgroups (`group/sub`).
    pub owner: String,
    pub name: String,
}

impl RepoRef {
    /// From a git remote URL (ssh scp-style, `ssh://`, `https://`).
    pub fn from_remote(remote: &str) -> Option<Self> {
        let (host, path) = gd_git::split_remote(remote)?;
        let (owner, name) = path.rsplit_once('/')?;
        Some(Self {
            host,
            owner: owner.to_string(),
            name: name.to_string(),
        })
    }

    /// `owner/name`, as in the web URL.
    pub fn full_path(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    /// GitLab's project id: the full path with `/` encoded.
    pub(crate) fn gitlab_id(&self) -> String {
        utf8_percent_encode(&self.full_path(), PATH_SEGMENT).to_string()
    }
}

#[cfg(test)]
#[path = "tests/repo_ref.test.rs"]
mod tests;
