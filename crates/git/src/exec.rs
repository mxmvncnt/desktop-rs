//! Thin `std::process` wrapper around the `git` binary.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, thiserror::Error)]
pub enum GitError {
    /// git exited non-zero; `stderr` is the trimmed message for display.
    #[error("{stderr}")]
    Failed { args: String, stderr: String },
    #[error("failed to run git: {0}")]
    Spawn(String),
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Lib(String),
}

impl From<git2::Error> for GitError {
    fn from(e: git2::Error) -> Self {
        Self::Lib(e.message().to_string())
    }
}

pub(crate) struct Git {
    cwd: PathBuf,
    args: Vec<OsString>,
    stdin: Option<String>,
    network: bool,
    env: Vec<(OsString, OsString)>,
}

/// Folders granted through the file-chooser portal live on the document
/// portal's FUSE mount (`/run/flatpak/doc/<id>/<name>`) inside a Flatpak
/// sandbox. `getcwd()` fails there, which makes git die ("unable to get
/// current working directory") whenever it runs inside such a folder. Run it
/// from `/` instead and point it at the repository explicitly.
fn portal_layout(cwd: &Path, args: &[OsString]) -> Option<[OsString; 2]> {
    if !cwd.starts_with("/run/flatpak/doc") || args.first().is_some_and(|a| a == "clone" || a == "init") {
        return None;
    }
    let git_dir = git2::Repository::open(cwd).ok()?.path().to_path_buf();
    let mut git_dir_arg = OsString::from("--git-dir=");
    git_dir_arg.push(git_dir);
    let mut work_tree_arg = OsString::from("--work-tree=");
    work_tree_arg.push(cwd);
    Some([git_dir_arg, work_tree_arg])
}

impl Git {
    pub fn new(cwd: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            args: Vec::new(),
            stdin: None,
            network: false,
            env: Vec::new(),
        }
    }

    pub fn arg(mut self, a: impl AsRef<OsStr>) -> Self {
        self.args.push(a.as_ref().to_os_string());
        self
    }

    pub fn args<I, S>(mut self, it: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args
            .extend(it.into_iter().map(|s| s.as_ref().to_os_string()));
        self
    }

    pub fn stdin(mut self, s: String) -> Self {
        self.stdin = Some(s);
        self
    }

    /// Marks a network operation: never prompt on the (absent) terminal.
    /// Credentials must come from a credential helper / ssh-agent.
    pub fn network(mut self) -> Self {
        self.network = true;
        self
    }

    /// Set an environment variable for this invocation only.
    pub fn env(mut self, k: impl AsRef<OsStr>, v: impl AsRef<OsStr>) -> Self {
        self.env.push((k.as_ref().to_os_string(), v.as_ref().to_os_string()));
        self
    }

    /// Never open an editor (merge/rebase/cherry-pick/revert `--continue`
    /// keep the prepared message).
    pub fn no_editor(self) -> Self {
        self.env("GIT_EDITOR", "true").env("GIT_SEQUENCE_EDITOR", "true")
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new("git");
        let portal = portal_layout(&self.cwd, &self.args);
        cmd.current_dir(portal.as_ref().map_or(self.cwd.as_path(), |_| Path::new("/")))
            .env("GIT_OPTIONAL_LOCKS", "0")
            .args(portal.iter().flatten())
            .args(&self.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        if self.network {
            cmd.env("GIT_TERMINAL_PROMPT", "0");
            for (k, v) in crate::credentials::env() {
                cmd.env(k, v);
            }
        }
        cmd.stdin(if self.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        cmd
    }

    /// Run, discarding stdout.
    pub fn exec(self) -> Result<(), GitError> {
        self.run_bytes().map(drop)
    }

    /// Run and return stdout as (lossy) UTF-8.
    pub fn run(self) -> Result<String, GitError> {
        self.run_bytes()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    }

    fn run_bytes(self) -> Result<Vec<u8>, GitError> {
        let start = std::time::Instant::now();
        let mut child = self
            .command()
            .spawn()
            .map_err(|e| GitError::Spawn(e.to_string()))?;
        if let Some(input) = &self.stdin
            && let Some(mut stdin) = child.stdin.take()
        {
            stdin.write_all(input.as_bytes()).map_err(|e| GitError::Io(e.to_string()))?;
        }
        let out = child
            .wait_with_output()
            .map_err(|e| GitError::Spawn(e.to_string()))?;
        let args = self
            .args
            .iter()
            .map(|a| a.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        log::debug!("git {args} ({:?})", start.elapsed());
        if out.status.success() {
            Ok(out.stdout)
        } else {
            let mut stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if stderr.is_empty() {
                // e.g. `stash apply` reports CONFLICT lines on stdout.
                stderr = String::from_utf8_lossy(&out.stdout).trim().to_string();
            }
            Err(GitError::Failed {
                args,
                stderr: if stderr.is_empty() {
                    format!("git exited with {}", out.status)
                } else {
                    stderr
                },
            })
        }
    }
}
