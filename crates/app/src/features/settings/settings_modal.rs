//! Settings modal: a sidebar of categories and the selected category's page.
//! Esc, the close button, a backdrop click, or the toolbar gear closes it.

use std::rc::Rc;

use gd_ui::themes::ColorThemes;
use gpui::{Context, Entity, IntoElement, RenderOnce};

use super::about_page::AboutPage;
use super::appearance_page::AppearancePage;
use super::diff_page::DiffPage;
use super::{SettingsPage, ThemeMode};
use crate::components::{NavItem, SettingsShell};
use crate::features::accounts::{AccountsPage, AccountsStore};
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct SettingsModal {
    page: SettingsPage,
    theme: ThemeMode,
    color_theme: String,
    ws: Entity<Workspace>,
    accounts: Entity<AccountsStore>,
}

impl SettingsModal {
    pub fn open(ws: &Workspace, cx: &mut Context<Workspace>) -> Self {
        let s = &ws.settings;
        Self {
            page: ws.settings_page,
            theme: s.theme,
            color_theme: s.color_theme_id(ColorThemes::of(cx)).to_string(),
            ws: cx.entity(),
            accounts: ws.accounts.clone(),
        }
    }
}

impl RenderOnce for SettingsModal {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let item = |p: SettingsPage| {
            let ws = self.ws.clone();
            NavItem {
                label: p.label(),
                icon: p.icon(),
                active: p == self.page,
                on_click: Rc::new(move |_, cx| ws.update(cx, |this, cx| this.set_settings_page(p, cx))),
            }
        };
        let [top @ .., about] = SettingsPage::ALL;
        let body = match self.page {
            SettingsPage::Appearance => {
                AppearancePage { mode: self.theme, color_theme: self.color_theme.clone(), ws: self.ws.clone() }.into_any_element()
            }
            SettingsPage::Diff => DiffPage { ws: self.ws.clone() }.into_any_element(),
            SettingsPage::About => AboutPage.into_any_element(),
            SettingsPage::Accounts => AccountsPage { store: self.accounts.clone() }.into_any_element(),
        };
        let ws = self.ws.clone();
        SettingsShell {
            title: self.page.label().into(),
            nav: top.into_iter().map(item).collect(),
            nav_bottom: vec![item(about)],
            body,
            on_close: Rc::new(move |_, cx| ws.update(cx, |this, cx| this.toggle_settings(cx))),
        }
    }
}
