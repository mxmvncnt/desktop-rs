//! `PaneView`: the sidebar or the main pane as its own cached view. A scroll
//! inside one re-renders only that one (lists notify the view they are in);
//! any workspace notify still re-renders both.

use gpui::{
    AnyElement, Context, IntoElement, ParentElement, Render, Styled, Subscription, WeakEntity,
    Window, div,
};

use super::state::Workspace;
use crate::features::changes::Sidebar;
use crate::repo::Tab;

#[derive(Clone, Copy)]
pub enum Part {
    Sidebar,
    Main,
}

pub struct PaneView {
    ws: WeakEntity<Workspace>,
    part: Part,
    /// Set on first render: the workspace entity doesn't exist yet when it builds its panes.
    ws_sub: Option<Subscription>,
}

impl Render for PaneView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(ws) = self.ws.upgrade() else {
            return div();
        };
        self.ws_sub
            .get_or_insert_with(|| cx.observe(&ws, |_, _, cx| cx.notify()));
        let part = self.part;
        let content = ws.update(cx, |ws, cx| -> Option<AnyElement> {
            let repo = ws.repo.clone()?;
            let (tab, pr_open) = {
                let r = repo.read(cx);
                (r.tab, r.pr.open)
            };
            Some(match part {
                Part::Sidebar => Sidebar::new(ws, &repo, cx).into_any_element(),
                Part::Main if pr_open => ws.render_pr_preview(&repo, cx),
                Part::Main if tab == Tab::Changes => ws.render_changes_main(&repo, cx),
                Part::Main => ws.render_history_main(&repo, cx),
            })
        });
        div().size_full().flex().children(content)
    }
}

impl PaneView {
    pub fn new(ws: WeakEntity<Workspace>, part: Part) -> Self {
        Self {
            ws,
            part,
            ws_sub: None,
        }
    }
}
