//! The root "template": toolbar, body (sidebar + main pane or welcome) and
//! the overlay stack.

use gd_ui::Theme;
use gpui::{
    AnyElement, Context, DragMoveEvent, MouseButton, InteractiveElement, IntoElement, ParentElement, Render, Styled, Window,
    div, px,
};

use super::{actions, overlay::Overlay, state::Workspace};
use crate::actions::{WORKSPACE, WORKSPACE_MENU};
use crate::components::MenuView;
use crate::features::branches::BranchDialogView;
use crate::features::changes::{Sidebar, SidebarResize};
use crate::features::diff::DiffOptionsMenu;
use crate::features::history::HistoryDialogView;
use crate::features::menus::DiscardDialogView;
use crate::features::merge::{InProgressBanner, MergeDialogView};
use crate::features::notices::NoticeBanner;
use crate::features::pickers::PopoverLayer;
use crate::features::settings::SettingsModal;
use crate::features::stashes::StashDialogView;
use crate::features::toolbar::Toolbar;
use crate::features::welcome::WelcomeScreen;
use crate::platform::Chrome;
use crate::repo::Tab;

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        crate::frame_debug::on_render();
        gd_ui::motion::begin_frame(cx);
        let chrome = Chrome::detect(window);
        self.corner_radius = chrome.radius();
        let body = self.body(cx);
        let overlays = self.overlays(cx);
        let in_progress = InProgressBanner::open(self, cx);
        let toolbar = Toolbar::new(self, cx.entity(), chrome, cx);

        let root = div()
            .id("workspace")
            .on_mouse_move(|_, _, _| crate::frame_debug::on_input())
            .on_drag_move(cx.listener(|this, e: &DragMoveEvent<SidebarResize>, _, cx| {
                this.drag_sidebar_to(e.event.position.x.as_f32(), cx)
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, _| { this.finish_sidebar_drag(); this.commit_msg.end_drag(); this.commit_sel.drag = false; }))
            .key_context(match self.overlays.last() {
                None | Some(Overlay::Popover(_)) => WORKSPACE,
                Some(_) => WORKSPACE_MENU,
            })
            .track_focus(&self.focus);
        let root = actions::bind(root, cx)
            .size_full()
            .flex()
            .flex_col()
            .relative()
            .bg(theme.bg)
            .font_family(theme.font_sans.clone())
            .text_size(px(13.0))
            .text_color(theme.text)
            .rounded(px(chrome.radius()))
            .child(toolbar)
            .children(in_progress)
            .child(body)
            .children(overlays);
        crate::platform::frame(root.into_any_element(), chrome, window, cx)
    }
}

impl Workspace {
    /// Sidebar + main pane for the open repo, or the welcome screen.
    fn body(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(repo) = self.repo.clone() else {
            return WelcomeScreen { ws: cx.entity() }.into_any_element();
        };
        let (tab, pr_open) = {
            let r = repo.read(cx);
            (r.tab, r.pr.open)
        };
        div()
            .flex()
            .flex_1()
            .min_h_0()
            .child(Sidebar::new(self, &repo, cx))
            .child(match tab {
                _ if pr_open => self.render_pr_preview(&repo, cx).into_any_element(),
                Tab::Changes => self.render_changes_main(&repo, cx).into_any_element(),
                Tab::History => self.render_history_main(&repo, cx).into_any_element(),
            })
            .into_any_element()
    }

    /// Notice banner, then the open overlays in stack order (last paints on top).
    fn overlays(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let mut out: Vec<AnyElement> = NoticeBanner::open(self, cx).into_iter().map(IntoElement::into_any_element).collect();
        let ws = cx.entity();
        for overlay in self.overlays.clone() {
            out.extend(match overlay {
                Overlay::Popover(p) => Some(PopoverLayer::open(self, p, cx).into_any_element()),
                Overlay::Menu(menu) => {
                    Some(MenuView { id: ("menu", self.overlay_gen).into(), menu, ws: ws.clone() }.into_any_element())
                }
                Overlay::DiffOptions(pos) => Some(DiffOptionsMenu::new(self, pos, ws.clone()).into_any_element()),
                Overlay::Settings => Some(SettingsModal::open(self, cx).into_any_element()),
                Overlay::Discard(d) => Some(DiscardDialogView::new(d, ws.clone()).into_any_element()),
                Overlay::History(d) => Some(HistoryDialogView::new(self, d, cx).into_any_element()),
                Overlay::Stash(d) => StashDialogView::open(self, d, cx).map(IntoElement::into_any_element),
                Overlay::Merge(d) => MergeDialogView::open(self, d, cx).map(IntoElement::into_any_element),
                Overlay::ForcePush => Some(self.render_force_push(cx)),
                Overlay::Branch(d) => BranchDialogView::open(self, d, cx).map(IntoElement::into_any_element),
            });
        }
        out
    }
}
