//! "Git identity" section of the Accounts page: the global identity.

use gd_ui::Button;
use gpui::{App, Entity, IntoElement, RenderOnce, Window};

use super::identity_block::IdentityBlock;
use super::state::{AccountInputs, AccountsStore, IdentityState};

#[derive(IntoElement)]
pub struct IdentitySection {
    pub identity: IdentityState,
    pub inputs: AccountInputs,
    pub store: Entity<AccountsStore>,
}

impl RenderOnce for IdentitySection {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let store = self.store;
        let inp = &self.inputs;
        IdentityBlock::new(
            "Git identity",
            "Used for your commits in every repository (global git config: user.name / user.email). A repository can override it in its settings.",
            ("identity-global-name", "identity-global-email"),
            (&inp.global_name, &inp.global_email),
        )
        .button(Button::new("identity-global-save").primary().compact().label("Save").on_click(move |_, _, cx| {
            store.update(cx, |this, cx| this.save_global_identity(cx))
        }))
        .status(self.identity.message)
    }
}
