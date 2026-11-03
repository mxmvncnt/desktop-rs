//! The repository settings modal: the shared settings layout with this feature's pages.

use std::rc::Rc;

use gd_git::Repository;
use gpui::{App, Context, Entity, IntoElement, RenderOnce, Window};

use super::config_page::ConfigPage;
use super::ignore_page::IgnorePage;
use super::info_page::InfoPage;
use super::remotes_page::RemotesPage;
use super::state::{RepoSettingsPage, RepoSettingsStore};
use crate::components::{NavItem, SettingsShell};
use crate::features::accounts::{AccountsStore, RepoAccountBlock, RepoIdentity};
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct RepoSettingsModal {
    repo: Repository,
    branch: String,
    store: Entity<RepoSettingsStore>,
    accounts: Entity<AccountsStore>,
    ws: Entity<Workspace>,
}

impl RepoSettingsModal {
    pub fn open(ws: &Workspace, cx: &mut Context<Workspace>) -> Option<Self> {
        let model = ws.repo.as_ref()?.read(cx);
        Some(Self {
            repo: model.repo.clone(),
            branch: model.branch_name().to_string(),
            store: ws.repo_settings.clone(),
            accounts: ws.accounts.clone(),
            ws: cx.entity(),
        })
    }
}

impl RenderOnce for RepoSettingsModal {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let s = self.store.read(cx);
        let page = s.page;
        let nav = RepoSettingsPage::ALL.map(|p| {
            let ws = self.ws.clone();
            NavItem {
                label: p.label(),
                icon: p.icon(),
                active: p == page,
                on_click: Rc::new(move |_, cx| {
                    ws.update(cx, |this, cx| this.set_repo_settings_page(p, cx))
                }),
            }
        });
        let a = self.accounts.read(cx);
        let title = format!("{} · {}", page.label(), self.repo.name());
        let body = match page {
            RepoSettingsPage::Info => InfoPage {
                repo: self.repo,
                branch: self.branch,
                info: s.info.clone(),
                error: s.error.clone(),
            }
            .into_any_element(),
            RepoSettingsPage::Remotes => RemotesPage {
                remotes: s.remotes.clone(),
                edit: s.edit.clone(),
                error: s.error.clone(),
                inputs: s.inputs.clone(),
                store: self.store,
            }
            .into_any_element(),
            RepoSettingsPage::Identity => RepoIdentity {
                identity: a.identity.clone(),
                inputs: a.inputs.clone(),
                repo: self.repo,
                store: self.accounts,
            }
            .into_any_element(),
            RepoSettingsPage::Ignore => IgnorePage {
                inputs: s.inputs.clone(),
                error: s.error.clone(),
                notice: s.notice,
                store: self.store,
            }
            .into_any_element(),
            RepoSettingsPage::Config => ConfigPage {
                entries: s.config.clone(),
                edit: s.edit.clone(),
                error: s.error.clone(),
                notice: s.notice,
                inputs: s.inputs.clone(),
                store: self.store,
            }
            .into_any_element(),
            RepoSettingsPage::Account => RepoAccountBlock {
                repo_path: self.repo.root().to_path_buf(),
                remote_host: a.identity.remote_host.clone(),
                accounts: a.accounts.clone(),
                picks: a.repo_accounts.clone(),
                store: self.accounts,
            }
            .into_any_element(),
        };
        let ws = self.ws;
        SettingsShell {
            title: title.into(),
            nav: nav.into(),
            nav_bottom: Vec::new(),
            body,
            on_close: Rc::new(move |w, cx| ws.update(cx, |this, cx| this.close_overlay(w, cx))),
        }
    }
}
