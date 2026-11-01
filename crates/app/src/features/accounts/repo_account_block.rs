//! "Account for this repository": Automatic (by remote host) or one of the accounts.

use std::path::PathBuf;

use gd_ui::{Button, Theme};
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

use super::model::Account;
use super::repo_account::{RepoAccounts, account_for_repo};
use super::state::AccountsStore;

#[derive(IntoElement)]
pub struct RepoAccountBlock {
    pub repo_path: PathBuf,
    pub remote_host: Option<String>,
    pub accounts: Vec<Account>,
    pub picks: RepoAccounts,
    pub store: Entity<AccountsStore>,
}

impl RenderOnce for RepoAccountBlock {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let picked = self.picks.get(self.repo_path.to_string_lossy().as_ref()).filter(|id| self.accounts.iter().any(|a| &a.id == *id));
        let in_use = match &self.remote_host {
            Some(host) => match account_for_repo(&self.accounts, &self.picks, &self.repo_path, host) {
                Some(a) => format!("In use: {} ({})", a.name, a.host),
                None => format!("No account for {host}. Add one, or pick another account."),
            },
            None => "This repository has no remote.".into(),
        };
        let choices = std::iter::once((None, "Automatic".to_string()))
            .chain(self.accounts.iter().map(|a| (Some(a.id.clone()), format!("{} ({})", a.name, a.host))))
            .map(|(id, label)| {
                let (store, path) = (self.store.clone(), self.repo_path.clone());
                let button = Button::new(format!("repo-account-{}", id.as_deref().unwrap_or("auto"))).compact().label(label);
                let button = if id.as_ref() == picked { button.primary() } else { button };
                button.on_click(move |_, _, cx| store.update(cx, |this, cx| this.set_repo_account(&path, id.clone(), cx)))
            });
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(div().font_weight(gpui::FontWeight::MEDIUM).child("Account for this repository"))
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.text_faint)
                    .child("Used for CI and API requests. Automatic takes the first account whose host is the remote's host."),
            )
            .child(div().flex().flex_wrap().gap(px(8.0)).children(choices))
            .child(div().text_size(px(12.0)).text_color(theme.text_muted).child(in_use))
    }
}
