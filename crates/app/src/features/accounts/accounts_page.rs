//! Settings → Accounts: git identity, then provider accounts (or the
//! add/edit form while it is open). Scrolls when taller than the modal.

use gd_git::Repository;
use gpui::{
    App, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled, Window, div, px,
};

use super::account_form::AccountForm;
use super::account_list::AccountList;
use super::identity_section::IdentitySection;
use super::state::AccountsStore;

#[derive(IntoElement)]
pub struct AccountsPage {
    pub store: Entity<AccountsStore>,
    /// The open repository, for the per-repository identity override.
    pub repo: Option<Repository>,
}

impl RenderOnce for AccountsPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let s = self.store.read(cx);
        let (store, inputs) = (self.store.clone(), s.inputs.clone());
        // The form replaces the whole page so it is fully visible.
        let content = match s.form.clone() {
            Some(form) => AccountForm { form, inputs, store }.into_any_element(),
            None => div()
                .flex()
                .flex_col()
                .gap(px(22.0))
                .child(IdentitySection { identity: s.identity.clone(), inputs, repo: self.repo, store: store.clone() })
                .child(div().h(px(1.0)).bg(gd_ui::Theme::of(cx).border))
                .child(AccountList { accounts: s.accounts.clone(), store })
                .into_any_element(),
        };
        div()
            .id("settings-accounts-scroll")
            .size_full()
            .overflow_y_scroll()
            .child(
                div().pr(px(8.0)).child(content),
            )
    }
}
