//! Fetch / pull / push on [`RepoModel`].

use gpui::Context;

use crate::repo::{Op, RemoteOp, RepoModel};

impl RepoModel {
    pub fn remote(&mut self, op: RemoteOp, cx: &mut Context<Self>) {
        if self.remote_op.is_some() {
            return;
        }
        let (branch, has_upstream) = match &self.work.status {
            Some(s) => (s.branch.clone().unwrap_or_default(), s.upstream.is_some()),
            None => (String::new(), false),
        };
        self.run(
            Op { remote: Some(op), ..Op::default() },
            move |r| match op {
                RemoteOp::Fetch => r.fetch().map(|_| true),
                RemoteOp::Pull => r.pull().map(|_| true),
                // Fetch first: if the remote moved, skip the push so the
                // refreshed toolbar offers Pull, then Push after the merge.
                RemoteOp::Push if has_upstream => r.push_if_up_to_date(&branch),
                RemoteOp::Push => r.push(&branch, false).map(|_| true),
            },
            move |this, done, _| this.notice = Some(done_message(op, done).into()),
            cx,
        );
    }
}

fn done_message(op: RemoteOp, done: bool) -> &'static str {
    match (op, done) {
        (RemoteOp::Push, false) => "Remote has new commits: pull to merge them, then push",
        (RemoteOp::Fetch, _) => "Fetched from remote",
        (RemoteOp::Pull, _) => "Pulled from remote",
        (RemoteOp::Push, true) => "Pushed to remote",
    }
}
