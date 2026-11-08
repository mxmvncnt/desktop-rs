//! The root "template": toolbar, body (sidebar + main pane or welcome) and
//! the overlay stack.

use gd_ui::{Theme, motion};
use gpui::{
    AnyElement, Context, DragMoveEvent, InteractiveElement, IntoElement, MouseButton,
    NavigationDirection, ParentElement, Render, StyleRefinement, Styled, Window, div, px,
};

use super::{actions, overlay::Overlay, state::Workspace};
use crate::actions::{WORKSPACE, WORKSPACE_MENU};
use crate::components::MenuView;
use crate::features::branches::BranchDialogView;
use crate::features::changes::SidebarResize;
use crate::features::ci::CiPage;
use crate::features::diff::DiffOptionsMenu;
use crate::features::history::HistoryDialogView;
use crate::features::menus::DiscardDialogView;
use crate::features::merge::{InProgressBanner, MergeDialogView};
use crate::features::notices::NoticeBanner;
use crate::features::pickers::PopoverLayer;
use crate::features::repo_settings::RepoSettingsModal;
use crate::features::settings::SettingsModal;
use crate::features::stashes::StashDialogView;
use crate::features::toolbar::Toolbar;
use crate::features::welcome::WelcomeScreen;
use crate::platform::Chrome;

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        crate::frame_debug::on_render();
        gd_ui::motion::begin_frame(cx);
        let chrome = Chrome::detect(window);
        self.corner_radius = chrome.radius();
        self.backdrop
            .update(self.background_appearance(), chrome, window);
        let body = self.body(cx);
        let overlays = self.overlays(cx);
        let in_progress = InProgressBanner::open(self, cx);
        let toolbar = Toolbar::new(self, cx.entity(), chrome, cx);

        let root = div()
            .id("workspace")
            .on_mouse_move(|_, _, _| crate::frame_debug::on_input())
            .on_drag_move(
                cx.listener(|this, e: &DragMoveEvent<SidebarResize>, _, cx| {
                    this.drag_sidebar_to(e.event.position.x.as_f32(), cx)
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.finish_sidebar_drag();
                    this.commit_msg.end_drag();
                    this.commit_sel.drag = false;
                    this.ci.update(cx, |ci, _| ci.log.dragging = false);
                }),
            )
            .on_mouse_up(
                MouseButton::Navigate(NavigationDirection::Back),
                cx.listener(|this, _, w, cx| this.dismiss(w, cx)),
            )
            .on_mouse_up(
                MouseButton::Navigate(NavigationDirection::Forward),
                cx.listener(|this, _, _, cx| {
                    this.forward_ci(cx);
                    this.forward_pulls(cx);
                }),
            )
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
            .font_family(theme.font_sans.clone())
            .text_size(px(13.0))
            .text_color(theme.text)
            .rounded(px(chrome.radius()))
            .child(toolbar)
            .children(in_progress.map(|b| div().bg(theme.bg).child(b)))
            .child(body)
            .children(overlays);
        crate::platform::frame(root.into_any_element(), chrome, window, cx)
    }
}

impl Workspace {
    /// Sidebar + main pane for the open repo, or the welcome screen. Each plane paints its own
    /// background and none sits on another, so translucent surfaces (theme `opacity`) don't stack.
    fn body(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::of(cx);
        let radius = px(self.corner_radius);
        let plane = div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(theme.bg)
            .rounded_b(radius);
        if self.repo.is_none() {
            return plane
                .child(WelcomeScreen { ws: cx.entity() })
                .into_any_element();
        }
        // Opening and closing the CI page swap the body: whichever appears fades in (the one that goes is gone at once).
        if self.ci.read(cx).open {
            let page = div().flex_1().min_h_0().flex().flex_col().child(CiPage {
                store: self.ci.clone(),
                ws: cx.entity(),
            });
            return plane
                .child(motion::fade_quick("ci-page-in", page))
                .into_any_element();
        }
        if self.pulls.read(cx).open {
            let page = div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .child(self.render_pulls_page(cx));
            return plane
                .child(motion::fade_quick("pulls-page-in", page))
                .into_any_element();
        }
        // Cached so a scroll in one pane doesn't rebuild the other (see `pane_view.rs`).
        // The backgrounds stay outside the fades, so the desktop never shows through mid-fade.
        let full = || StyleRefinement::default().size_full();
        let sidebar = div()
            .flex_none()
            .w(px(self.sidebar_width()))
            .h_full()
            .bg(theme.surface)
            .rounded_bl(radius)
            .child(motion::fade_quick(
                "sidebar-in",
                div()
                    .size_full()
                    .child(self.sidebar_pane.clone().cached(full())),
            ));
        let main = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(theme.bg)
            .rounded_br(radius)
            .child(motion::fade_quick(
                "main-in",
                div()
                    .size_full()
                    .child(self.main_pane.clone().cached(full())),
            ));
        div()
            .flex()
            .flex_1()
            .min_h_0()
            .child(sidebar)
            .child(main)
            .into_any_element()
    }

    /// Notice banner, then the open overlays in stack order (last paints on top).
    fn overlays(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let mut out: Vec<AnyElement> = NoticeBanner::open(self, cx)
            .into_iter()
            .map(IntoElement::into_any_element)
            .collect();
        let ws = cx.entity();
        for overlay in self.overlays.clone() {
            out.extend(match overlay {
                Overlay::Popover(p) => Some(PopoverLayer::open(self, p, cx).into_any_element()),
                Overlay::Menu(menu) => Some(
                    MenuView {
                        id: ("menu", self.overlay_gen).into(),
                        menu,
                        ws: ws.clone(),
                    }
                    .into_any_element(),
                ),
                Overlay::DiffOptions(pos) => {
                    Some(DiffOptionsMenu::new(self, pos, ws.clone()).into_any_element())
                }
                Overlay::Settings => Some(SettingsModal::open(self, cx).into_any_element()),
                Overlay::RepoSettings => {
                    RepoSettingsModal::open(self, cx).map(IntoElement::into_any_element)
                }
                Overlay::Discard(d) => {
                    Some(DiscardDialogView::new(d, ws.clone()).into_any_element())
                }
                Overlay::History(d) => Some(HistoryDialogView::new(self, d, cx).into_any_element()),
                Overlay::Stash(d) => {
                    StashDialogView::open(self, d, cx).map(IntoElement::into_any_element)
                }
                Overlay::Merge(d) => {
                    MergeDialogView::open(self, d, cx).map(IntoElement::into_any_element)
                }
                Overlay::ForcePush => Some(self.render_force_push(cx)),
                Overlay::Branch(d) => {
                    BranchDialogView::open(self, d, cx).map(IntoElement::into_any_element)
                }
            });
        }
        out
    }
}
