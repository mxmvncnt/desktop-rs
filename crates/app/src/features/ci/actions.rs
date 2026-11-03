//! Retry, cancel and play: which buttons a job gets, and running a request off-thread.

use gd_forge::{Capabilities, Job, JobId, PipelineId, Status};
use gd_ui::Icon;
use gpui::{AppContext, Context};

use super::state::{CiEvent, CiStore};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CiAction {
    RetryPipeline(PipelineId),
    CancelPipeline(PipelineId),
    RetryJob(JobId),
    CancelJob(JobId),
    PlayJob(JobId),
}

impl CiAction {
    fn pipeline(self) -> PipelineId {
        match self {
            Self::RetryPipeline(id) | Self::CancelPipeline(id) => id,
            Self::RetryJob(job) | Self::CancelJob(job) | Self::PlayJob(job) => job.pipeline,
        }
    }

    fn done(self) -> &'static str {
        match self {
            Self::RetryPipeline(_) => "Retrying the failed jobs",
            Self::CancelPipeline(_) => "Pipeline canceled",
            Self::RetryJob(_) => "Retrying the job",
            Self::CancelJob(_) => "Job canceled",
            Self::PlayJob(_) => "Job started",
        }
    }
}

/// The buttons of a job (card and sidebar): what the platform can do with its status.
pub fn job_actions(job: &Job, caps: Capabilities) -> Vec<(CiAction, Icon, &'static str)> {
    let running = matches!(
        job.status,
        Status::Running | Status::Pending | Status::Preparing | Status::WaitingForResource
    );
    let mut actions = Vec::new();
    if caps.play_job && job.status == Status::Manual {
        actions.push((CiAction::PlayJob(job.id), Icon::Play, "Play"));
    }
    if caps.retry_job && job.status.is_finished() {
        actions.push((CiAction::RetryJob(job.id), Icon::Restart, "Retry"));
    }
    if caps.cancel_job && running {
        actions.push((CiAction::CancelJob(job.id), Icon::Stop, "Cancel"));
    }
    actions
}

impl CiStore {
    /// Send the request, tell the user how it went, then reload what it changed.
    pub fn run_action(&mut self, action: CiAction, cx: &mut Context<Self>) {
        let Some(target) = self.ready().cloned() else {
            return;
        };
        if self.acting {
            return;
        }
        self.acting = true;
        let epoch = self.epoch;
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_spawn(async move {
                    let (client, repo) = (target.client()?, &target.repo);
                    match action {
                        CiAction::RetryPipeline(id) => client.retry_pipeline(repo, &id),
                        CiAction::CancelPipeline(id) => client.cancel_pipeline(repo, &id),
                        CiAction::RetryJob(job) => client.retry_job(repo, &job),
                        CiAction::CancelJob(job) => client.cancel_job(repo, &job),
                        CiAction::PlayJob(job) => client.play_job(repo, &job),
                    }
                })
                .await;
            this.update(cx, |this, cx| {
                if this.epoch != epoch {
                    return;
                }
                this.acting = false;
                match res {
                    Ok(()) => {
                        cx.emit(CiEvent::Notice(action.done().into()));
                        this.fetch_detail(action.pipeline(), cx);
                        this.load_list(false, cx);
                    }
                    Err(e) => cx.emit(CiEvent::Error(format!("CI request failed: {e}"))),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}

#[cfg(test)]
#[path = "tests/actions.test.rs"]
mod tests;
