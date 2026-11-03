//! Refresh while the page is visible and the window is active: every 10 s when something is running, else every minute.

use std::time::Duration;

use gpui::Context;

use super::state::{CiStore, CiView};

const LOG: Duration = Duration::from_secs(5);
const RUNNING: Duration = Duration::from_secs(10);
const IDLE: Duration = Duration::from_secs(60);

/// How long to wait before the next refresh: faster while a log is being followed.
pub fn poll_interval(running: bool, log: bool) -> Duration {
    match (running, log) {
        (true, true) => LOG,
        (true, false) => RUNNING,
        _ => IDLE,
    }
}

impl CiStore {
    pub(super) fn start_polling(&mut self, cx: &mut Context<Self>) {
        if self.poll.is_some() {
            return;
        }
        self.poll = Some(cx.spawn(async move |this, cx| {
            loop {
                let Ok(wait) = this.update(cx, |s, _| {
                    poll_interval(s.shows_running(), matches!(s.view, CiView::Job(_)))
                }) else {
                    break;
                };
                cx.background_executor().timer(wait).await;
                if this.update(cx, |s, cx| s.poll(cx)).is_err() {
                    break;
                }
            }
        }));
    }

    /// The window was (de)activated; coming back refreshes at once.
    pub fn set_focused(&mut self, focused: bool, cx: &mut Context<Self>) {
        self.focused = focused;
        if focused && self.open {
            self.poll(cx);
        }
    }

    /// Something on screen is still changing.
    fn shows_running(&self) -> bool {
        match self.view {
            CiView::List => self.pipelines.iter().any(|p| !p.status.is_finished()),
            CiView::Pipeline(id) => self
                .details
                .get(&id)
                .is_none_or(|d| !d.pipeline.status.is_finished()),
            CiView::Job(job) => self.job(job).is_none_or(|j| !j.status.is_finished()),
        }
    }

    fn poll(&mut self, cx: &mut Context<Self>) {
        if !self.open || !self.focused {
            return;
        }
        self.failed.clear();
        let running: Vec<_> = self
            .details
            .values()
            .filter(|d| !d.pipeline.status.is_finished())
            .map(|d| d.pipeline.id)
            .collect();
        match self.view {
            CiView::List => {
                self.load_list(false, cx);
                for id in running {
                    self.fetch_detail(id, cx);
                }
            }
            CiView::Pipeline(id) => self.fetch_detail(id, cx),
            CiView::Job(job) => {
                self.fetch_detail(job.pipeline, cx);
                if self.job(job).is_none_or(|j| !j.status.is_finished()) {
                    self.load_log(job, cx);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/polling.test.rs"]
mod tests;
