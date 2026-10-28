//! Values derived from the repo model for the sidebar, so the components
//! take typed props instead of a dozen locals.

use gd_ui::CheckState;
use gpui::SharedString;

use crate::repo::RepoModel;
use crate::utils::text::plural;

/// State of the "select all" checkbox and the count above the list.
#[derive(Clone, Copy)]
pub struct SelectionSummary {
    pub total: usize,
    /// Files not unchecked in the list.
    pub included: usize,
    pub all: CheckState,
}

impl SelectionSummary {
    pub fn of(r: &RepoModel) -> Self {
        let total = r.changes().len();
        let included = total - r.changes().iter().filter(|c| r.work.excluded.contains(&c.path)).count();
        let all = match included {
            0 => CheckState::Unchecked,
            i if i == total && r.work.partial.is_empty() => CheckState::Checked,
            _ => CheckState::Mixed,
        };
        Self { total, included, all }
    }
}

/// What the commit button shows and whether it is enabled.
pub struct CommitButton {
    pub label: SharedString,
    pub enabled: bool,
    /// The commit being amended was already pushed (force push needed).
    pub head_pushed: bool,
    /// Shown as the summary placeholder and used when the summary is empty.
    pub default_summary: Option<String>,
}

impl CommitButton {
    pub fn of(r: &RepoModel, included: usize, amending: bool, summary_typed: bool) -> Self {
        let merging = r.merging();
        let default_summary = r.default_summary(amending);
        let summary_ok = summary_typed || default_summary.is_some();
        let committing = r.committing;
        let conflicts = r.conflict_count();
        // Amending may change only the message, so no files are needed.
        let enabled = (included > 0 || amending) && summary_ok && !committing && conflicts == 0 && r.merge.busy.is_none();
        let label: SharedString = if amending {
            if committing { "Amending…".into() } else { "Amend last commit".into() }
        } else if committing {
            "Committing…".into()
        } else if conflicts > 0 {
            format!("Resolve {conflicts} conflict{} to commit", plural(conflicts as u32)).into()
        } else if merging {
            "Commit merge".into()
        } else {
            format!("Commit to {}", r.branch_name()).into()
        };
        let head_pushed = r.history.push_state_loaded && r.log.commits.first().is_some_and(|c| !r.is_unpushed(&c.sha));
        Self { label, enabled, head_pushed, default_summary }
    }
}
