use std::io::Read;
use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::error::{Error, Result};
use crate::model::*;
use crate::pull_model::*;
use crate::repo_ref::RepoRef;
use crate::{actions, forgejo, gitea_pulls, github_pulls, gitlab, gitlab_pulls};

pub(crate) const PAGE_SIZE: u32 = 20;
const MAX_LOG_BYTES: u64 = 64 * 1024 * 1024;
/// Page size of `get_all`, and the most pages it reads.
const ALL_PAGE: u32 = 100;
const MAX_PAGES: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Forge {
    /// github.com and GitHub Enterprise Server.
    GitHub,
    GitLab,
    Gitea,
    Forgejo,
}

impl Forge {
    pub fn capabilities(self) -> Capabilities {
        let none = Capabilities {
            retry_pipeline: false,
            retry_job: false,
            cancel_pipeline: true,
            cancel_job: false,
            play_job: false,
            job_logs: true,
            steps: false,
            stages: false,
            filter_user: false,
            line_ranges: false,
        };
        match self {
            Forge::GitHub => Capabilities {
                retry_pipeline: true,
                retry_job: true,
                steps: true,
                filter_user: true,
                line_ranges: true,
                ..none
            },
            Forge::Gitea => Capabilities {
                retry_pipeline: true,
                retry_job: true,
                steps: true,
                filter_user: true,
                ..none
            },
            Forge::GitLab => Capabilities {
                retry_pipeline: true,
                retry_job: true,
                cancel_job: true,
                play_job: true,
                stages: true,
                filter_user: true,
                line_ranges: true,
                ..none
            },
            // Forgejo: runs, jobs with `needs`, logs and run cancel only; its runs list has no user filter.
            Forge::Forgejo => Capabilities {
                stages: true,
                ..none
            },
        }
    }
}

/// A blocking API client for one host and token. Cheap to clone.
#[derive(Clone)]
pub struct Client {
    pub(crate) forge: Forge,
    base: String,
    token: String,
    agent: ureq::Agent,
}

impl Client {
    pub fn new(forge: Forge, host: &str, token: &str) -> Self {
        let base = match forge {
            Forge::GitHub if host == "github.com" => "https://api.github.com".to_string(),
            Forge::GitHub => format!("https://{host}/api/v3"),
            Forge::GitLab => format!("https://{host}/api/v4"),
            Forge::Gitea | Forge::Forgejo => format!("https://{host}/api/v1"),
        };
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build();
        Self {
            forge,
            base,
            token: token.to_string(),
            agent,
        }
    }

    pub fn capabilities(&self) -> Capabilities {
        self.forge.capabilities()
    }

    /// Runs, newest first, `PAGE_SIZE` per page; `page` starts at 1. List entries lack the commit title and,
    /// on GitLab, the user; `pipeline()` has them.
    pub fn pipelines(
        &self,
        repo: &RepoRef,
        filter: &PipelineFilter,
        page: u32,
    ) -> Result<Vec<Pipeline>> {
        match self.forge {
            Forge::GitHub | Forge::Gitea => actions::pipelines(self, repo, filter, page),
            Forge::GitLab => gitlab::pipelines(self, repo, filter, page),
            Forge::Forgejo => forgejo::pipelines(self, repo, filter, page),
        }
    }

    pub fn pipeline(&self, repo: &RepoRef, id: &PipelineId) -> Result<PipelineDetail> {
        match self.forge {
            Forge::GitHub | Forge::Gitea => actions::pipeline(self, repo, id),
            Forge::GitLab => gitlab::pipeline(self, repo, id),
            Forge::Forgejo => forgejo::pipeline(self, repo, id),
        }
    }

    /// The raw job log (ANSI colors included); see `parse_log`.
    pub fn job_log(&self, repo: &RepoRef, job: &JobId) -> Result<String> {
        match self.forge {
            Forge::GitHub | Forge::Gitea | Forge::Forgejo => actions::job_log(self, repo, job),
            Forge::GitLab => gitlab::job_log(self, repo, job),
        }
    }

    /// Re-run the failed jobs of a pipeline.
    pub fn retry_pipeline(&self, repo: &RepoRef, id: &PipelineId) -> Result<()> {
        match self.forge {
            Forge::GitHub | Forge::Gitea => actions::retry_pipeline(self, repo, id),
            Forge::GitLab => gitlab::post(self, repo, &format!("/pipelines/{}/retry", id.0)),
            Forge::Forgejo => Err(Error::Unsupported("retrying a pipeline")),
        }
    }

    pub fn cancel_pipeline(&self, repo: &RepoRef, id: &PipelineId) -> Result<()> {
        match self.forge {
            Forge::GitHub | Forge::Gitea | Forge::Forgejo => {
                actions::cancel_pipeline(self, repo, id)
            }
            Forge::GitLab => gitlab::post(self, repo, &format!("/pipelines/{}/cancel", id.0)),
        }
    }

    pub fn retry_job(&self, repo: &RepoRef, job: &JobId) -> Result<()> {
        match self.forge {
            Forge::GitHub | Forge::Gitea => actions::retry_job(self, repo, job),
            Forge::GitLab => gitlab::post(self, repo, &format!("/jobs/{}/retry", job.id)),
            Forge::Forgejo => Err(Error::Unsupported("retrying a job")),
        }
    }

    pub fn cancel_job(&self, repo: &RepoRef, job: &JobId) -> Result<()> {
        match self.forge {
            Forge::GitLab => gitlab::post(self, repo, &format!("/jobs/{}/cancel", job.id)),
            _ => Err(Error::Unsupported("cancelling a job")),
        }
    }

    /// Start a manual job.
    pub fn play_job(&self, repo: &RepoRef, job: &JobId) -> Result<()> {
        match self.forge {
            Forge::GitLab => gitlab::post(self, repo, &format!("/jobs/{}/play", job.id)),
            _ => Err(Error::Unsupported("playing a manual job")),
        }
    }

    /// Pull requests, most recently updated first, `PAGE_SIZE` per page; `page` starts at 1. On GitHub and Gitea
    /// a Closed or Merged filter leaves the other out of the page, so a page may be short with more after it.
    pub fn pulls(
        &self,
        repo: &RepoRef,
        filter: &PullFilter,
        page: u32,
    ) -> Result<Vec<PullRequest>> {
        match self.forge {
            Forge::GitHub => github_pulls::pulls(self, repo, filter, page),
            Forge::GitLab => gitlab_pulls::pulls(self, repo, filter, page),
            Forge::Gitea | Forge::Forgejo => gitea_pulls::pulls(self, repo, filter, page),
        }
    }

    /// One pull request, with what the list leaves out (GitLab `diff_refs`, GitHub comment count).
    pub fn pull(&self, repo: &RepoRef, number: u64) -> Result<PullRequest> {
        match self.forge {
            Forge::GitHub => github_pulls::pull(self, repo, number),
            Forge::GitLab => gitlab_pulls::pull(self, repo, number),
            Forge::Gitea | Forge::Forgejo => gitea_pulls::pull(self, repo, number),
        }
    }

    /// The comments and review threads of a pull request.
    pub fn conversation(&self, repo: &RepoRef, number: u64) -> Result<Conversation> {
        match self.forge {
            Forge::GitHub => github_pulls::conversation(self, repo, number),
            Forge::GitLab => gitlab_pulls::conversation(self, repo, number),
            Forge::Gitea | Forge::Forgejo => gitea_pulls::conversation(self, repo, number),
        }
    }

    /// A comment on the conversation.
    pub fn comment(&self, repo: &RepoRef, number: u64, body: &str) -> Result<()> {
        match self.forge {
            Forge::GitHub => github_pulls::comment(self, repo, number, body),
            Forge::GitLab => gitlab_pulls::comment(self, repo, number, body),
            Forge::Gitea | Forge::Forgejo => gitea_pulls::comment(self, repo, number, body),
        }
    }

    /// A review comment on lines of the diff of `pull`'s head. Without `Capabilities::line_ranges` it goes on the last line.
    pub fn line_comment(
        &self,
        repo: &RepoRef,
        pull: &PullRequest,
        target: &LineTarget,
        body: &str,
    ) -> Result<()> {
        match self.forge {
            Forge::GitHub => github_pulls::line_comment(self, repo, pull, target, body),
            Forge::GitLab => gitlab_pulls::line_comment(self, repo, pull, target, body),
            Forge::Gitea | Forge::Forgejo => {
                gitea_pulls::line_comment(self, repo, pull, target, body)
            }
        }
    }

    pub fn reply(
        &self,
        repo: &RepoRef,
        pull: &PullRequest,
        thread: &Thread,
        body: &str,
    ) -> Result<()> {
        match self.forge {
            Forge::GitHub => github_pulls::reply(self, repo, pull.number, thread, body),
            Forge::GitLab => gitlab_pulls::reply(self, repo, pull.number, thread, body),
            Forge::Gitea | Forge::Forgejo => gitea_pulls::reply(self, repo, pull, thread, body),
        }
    }

    pub(crate) fn call(
        &self,
        method: &str,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<ureq::Response> {
        let mut req = self
            .agent
            .request(method, &format!("{}{path}", self.base))
            .set("Authorization", &format!("Bearer {}", self.token))
            .set("User-Agent", "desktop-rs")
            .set(
                "Accept",
                "application/vnd.github+json, application/json, text/plain",
            )
            .set("X-GitHub-Api-Version", "2022-11-28");
        for (key, value) in query {
            req = req.query(key, value);
        }
        answer(req.call())
    }

    /// `method` with a JSON body; the answer is not read.
    pub(crate) fn send(&self, method: &str, path: &str, body: serde_json::Value) -> Result<()> {
        let req = self
            .agent
            .request(method, &format!("{}{path}", self.base))
            .set("Authorization", &format!("Bearer {}", self.token))
            .set("User-Agent", "desktop-rs")
            .set("Accept", "application/vnd.github+json, application/json")
            .set("X-GitHub-Api-Version", "2022-11-28");
        answer(req.send_json(body)).map(drop)
    }

    /// Every page of a list, up to `MAX_PAGES` of `ALL_PAGE`. GitHub and GitLab read `per_page`, Gitea `limit`
    /// (capped at 50 by default), so only a page under 50 or an empty one is known to be the last.
    pub(crate) fn get_all<T: DeserializeOwned>(&self, path: &str) -> Result<Vec<T>> {
        let mut all = Vec::new();
        for page in 1..=MAX_PAGES {
            let items: Vec<T> = self.get(
                path,
                &[
                    ("per_page", ALL_PAGE.to_string()),
                    ("limit", ALL_PAGE.to_string()),
                    ("page", page.to_string()),
                ],
            )?;
            let last = items.len() < 50;
            all.extend(items);
            if last {
                break;
            }
        }
        Ok(all)
    }

    pub(crate) fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        self.call("GET", path, query)?
            .into_json()
            .map_err(|e| Error::Parse(e.to_string()))
    }

    pub(crate) fn text(&self, path: &str) -> Result<String> {
        let mut bytes = Vec::new();
        self.call("GET", path, &[])?
            .into_reader()
            .take(MAX_LOG_BYTES)
            .read_to_end(&mut bytes)
            .map_err(|e| Error::Network(e.to_string()))?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

fn answer(res: std::result::Result<ureq::Response, ureq::Error>) -> Result<ureq::Response> {
    match res {
        Ok(resp) => Ok(resp),
        Err(ureq::Error::Status(code, resp)) => Err(status_error(code, resp)),
        Err(ureq::Error::Transport(t)) => Err(Error::Network(t.to_string())),
    }
}

fn status_error(code: u16, resp: ureq::Response) -> Error {
    let retry_after = resp.header("retry-after").and_then(|s| s.parse().ok());
    let limited = resp.header("x-ratelimit-remaining") == Some("0");
    match code {
        401 => Error::Unauthorized,
        403 if limited || retry_after.is_some() => Error::RateLimited(retry_after),
        403 => Error::Forbidden,
        404 => Error::NotFound,
        429 => Error::RateLimited(retry_after),
        _ => Error::Http {
            status: code,
            message: resp
                .into_string()
                .unwrap_or_default()
                .chars()
                .take(300)
                .collect(),
        },
    }
}

#[cfg(test)]
#[path = "tests/client.test.rs"]
mod tests;
