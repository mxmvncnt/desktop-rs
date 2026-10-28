//! `Workspace::new`: inputs, window subscriptions, first repository.

use std::path::PathBuf;
use std::time::Duration;

use gpui::{App, AppContext, Context, Entity, Focusable, Subscription, UniformListScrollHandle, Window};
use gpui_elements::editable_text::{EditableTextState, TextChanged};

use super::state::Workspace;
use crate::features::accounts::AccountsStore;
use crate::features::branches::BranchUi;
use crate::features::changes::CommitBox;
use crate::features::history::HistoryUi;
use crate::features::merge::MergeUi;
use crate::features::pickers::PopoverStore;
use crate::features::settings::{Settings, ThemeMode};
use crate::features::stashes::StashUi;

/// Skip the focus refresh when the watcher refreshed this recently.
const RECENT_WATCH_REFRESH: Duration = Duration::from_secs(1);

impl Workspace {
    pub fn new(open_path: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let ws = cx.weak_entity();
        let commit_box = cx.new(|cx| CommitBox::new(ws, cx));
        let pickers = cx.new(|cx| PopoverStore::new(cx));
        let p = pickers.read(cx);
        let inputs = [p.filter.clone(), p.name_input.clone(), p.url_input.clone()];
        let mut subs = input_subscriptions(&inputs, cx);
        subs.push(cx.observe(&pickers, |_, _, cx| cx.notify()));
        let accounts = cx.new(|cx| AccountsStore::new(cx));
        subs.push(cx.observe(&accounts, |_, _, cx| cx.notify()));
        subs.extend(window_subscriptions(window, cx));

        let mut this = Self {
            settings: Settings::load(),
            sidebar_width_dirty: false,
            settings_page: Default::default(),
            accounts,
            repo: None,
            overlays: Vec::new(),
            overlay_gen: 0,
            pickers,
            history_ui: HistoryUi::new(cx),
            stash_ui: StashUi::new(cx),
            merge_ui: MergeUi::new(cx),
            branch_ui: BranchUi::new(cx),
            startup_error: None,
            corner_radius: 0.0,
            commit_box,
            nav_list: None,
            changes_scroll: UniformListScrollHandle::new(),
            history_scroll: UniformListScrollHandle::new(),
            diff_scroll: UniformListScrollHandle::new(),
            diff_key: None,
            diff_view: Default::default(),
            diff_select: Default::default(),
            commit_msg: Default::default(),
            focus: cx.focus_handle(),
            notice_task: None,
            repo_sub: None,
            repo_event_sub: None,
            _subs: subs,
        };
        this.apply_theme(window, cx);
        if let Some(path) = open_path {
            this.add_repository(path, cx);
        } else if let Some(ix) = this.settings.selected {
            this.open_repository(ix, cx);
        }
        this
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus, cx);
    }
}

impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

/// Re-render when inputs change (commit button enablement, filtering).
fn input_subscriptions(
    inputs: &[Entity<EditableTextState>],
    cx: &mut Context<Workspace>,
) -> Vec<Subscription> {
    inputs
        .iter()
        .map(|input| {
            cx.subscribe(input, |this: &mut Workspace, _, _: &TextChanged, cx| {
                this.reset_popover_selection(cx);
                cx.notify()
            })
        })
        .collect()
}

fn window_subscriptions(window: &mut Window, cx: &mut Context<Workspace>) -> [Subscription; 2] {
    [
        // Desktop refreshes status when the window regains focus.
        cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                // The watcher already keeps us fresh; skip if it just refreshed.
                let recent = this.repo.as_ref().is_some_and(|r| {
                    r.read(cx).sync.last_watch_refresh.is_some_and(|t| t.elapsed() < RECENT_WATCH_REFRESH)
                });
                if !recent {
                    this.refresh(cx);
                }
            }
        }),
        cx.observe_window_appearance(window, |this, window, cx| {
            if this.settings.theme == ThemeMode::System {
                this.apply_theme(window, cx);
            }
        }),
    ]
}
