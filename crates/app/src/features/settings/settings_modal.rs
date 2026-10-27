//! Settings modal: a sidebar of categories and the selected category's page.
//! Esc, the close button, a backdrop click, or the toolbar gear closes it.

use std::rc::Rc;

use gd_ui::{Callback, Icon, divider, icon_button, modal, motion, overlay_card};
use gpui::{
    AnyElement, App, Context, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled, Window, div,
    px,
};

use crate::features::accounts::AccountsPage;
use super::appearance_page::AppearancePage;
use super::diff_page::DiffPage;
use super::settings_nav::SettingsNav;
use super::{SettingsPage, ThemeMode};
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct SettingsModal {
    page: SettingsPage,
    theme: ThemeMode,
    color_theme: String,
    diff: DiffPage,
    accounts: Option<AccountsPage>,
    ws: Entity<Workspace>,
}

impl SettingsModal {
    pub fn open(ws: &Workspace, cx: &mut Context<Workspace>) -> Self {
        let s = &ws.settings;
        Self {
            page: ws.settings_page,
            theme: s.theme,
            color_theme: s.color_theme_id().to_string(),
            diff: DiffPage::new(s, cx.entity()),
            accounts: (ws.settings_page == SettingsPage::Accounts).then(|| AccountsPage {
                store: ws.accounts.clone(),
                repo: ws.repo.as_ref().map(|r| r.read(cx).repo.clone()),
            }),
            ws: cx.entity(),
        }
    }

    fn body(self) -> AnyElement {
        match self.page {
            SettingsPage::Appearance => AppearancePage {
                mode: self.theme,
                color_theme: self.color_theme.clone(),
                ws: self.ws.clone(),
            }.into_any_element(),
            SettingsPage::Diff => self.diff.into_any_element(),
            SettingsPage::Accounts => self.accounts.map_or_else(|| div().into_any_element(), IntoElement::into_any_element),
        }
    }

    fn title_bar(&self) -> impl IntoElement + use<> {
        let ws = self.ws.clone();
        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .h(px(52.0))
            .pl(px(24.0))
            .pr(px(12.0))
            .child(div().text_size(px(15.0)).font_weight(gpui::FontWeight::SEMIBOLD).child(self.page.label()))
            .child(
                icon_button("settings-close", Icon::Close)
                    .compact()
                    .on_click(move |_, _, cx| close(&ws, cx)),
            )
    }

    fn card(self, cx: &mut App) -> gpui::Stateful<gpui::Div> {
        let (nav, title) = (SettingsNav { page: self.page, ws: self.ws.clone() }, self.title_bar());
        let body = self.body();
        overlay_card(cx)
            .id("settings-card")
            .occlude()
            .w(px(740.0))
            .h(px(560.0))
            .flex()
            .child(nav)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(title)
                    .child(divider(cx))
                    .child(div().flex_1().min_h_0().p(px(24.0)).child(body)),
            )
    }
}

impl RenderOnce for SettingsModal {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let instant = motion::handoff(&"settings-in".into(), cx);
        let ws = self.ws.clone();
        let on_cancel: Callback = Rc::new(move |_, cx| close(&ws, cx));
        modal(self.card(cx), Some(on_cancel), instant)
    }
}

fn close(ws: &Entity<Workspace>, cx: &mut App) {
    ws.update(cx, |this, cx| this.toggle_settings(cx))
}
