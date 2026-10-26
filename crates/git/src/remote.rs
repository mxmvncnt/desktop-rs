//! Remote URL parsing: host, repository path, hosting provider.

/// Where a pull request can be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Host {
    GitHub,
    GitLab,
    Bitbucket,
}

impl Host {
    pub fn name(self) -> &'static str {
        match self {
            Host::GitHub => "GitHub",
            Host::GitLab => "GitLab",
            Host::Bitbucket => "Bitbucket",
        }
    }
}

/// Split a git remote URL (ssh scp-style, `ssh://`, `https://`, `git://`)
/// into a lowercase host and the repository path (no `.git`, no slashes
/// around). `None` when the path is not `owner/repo`-shaped.
pub(crate) fn split_remote(remote: &str) -> Option<(String, String)> {
    let remote = remote.trim();
    let (host, path) = if let Some((_, rest)) = remote.split_once("://") {
        let (authority, path) = rest.split_once('/')?;
        let host = authority.rsplit('@').next()?;
        (host.split(':').next()?.to_string(), path.to_string())
    } else {
        // scp-like: [user@]host:owner/repo.git
        let (authority, path) = remote.split_once(':')?;
        (authority.rsplit('@').next()?.to_string(), path.to_string())
    };
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    if path.split('/').count() < 2 || path.is_empty() {
        return None;
    }
    Some((host.to_ascii_lowercase(), path.to_string()))
}

/// Which provider a remote URL belongs to.
pub fn host_of(remote: &str) -> Option<Host> {
    let (host, _) = split_remote(remote)?;
    if host.contains("github") {
        Some(Host::GitHub)
    } else if host.contains("gitlab") {
        Some(Host::GitLab)
    } else if host.contains("bitbucket") {
        Some(Host::Bitbucket)
    } else {
        None
    }
}

/// Convert a git remote URL into the repository's GitHub web URL. Hosts must
/// look like GitHub (`github.com`, or a GitHub Enterprise host containing "github").
pub fn github_web_url(remote: &str) -> Option<String> {
    let (host, path) = split_remote(remote)?;
    (host_of(remote) == Some(Host::GitHub)).then(|| format!("https://{host}/{path}"))
}
