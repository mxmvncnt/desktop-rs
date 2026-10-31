//! The identity of one repository: the local override editor and the identity in effect.

use gd_git::{EffectiveIdentity, IdentitySource, Repository};
use gd_ui::{Button, Theme};
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

use super::identity_block::IdentityBlock;
use super::state::{AccountInputs, AccountsStore, IdentityState};

#[derive(IntoElement)]
pub struct RepoIdentity {
    pub identity: IdentityState,
    pub inputs: AccountInputs,
    pub repo: Repository,
    pub store: Entity<AccountsStore>,
}

impl RenderOnce for RepoIdentity {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let (save, clear) = (self.store.clone(), self.store.clone());
        let (save_repo, clear_repo) = (self.repo.clone(), self.repo.clone());
        let inp = &self.inputs;
        let block = IdentityBlock::new(
            "Override for this repository",
            "Optional. Leave empty to use the global identity in this repository (local git config).",
            ("identity-local-name", "identity-local-email"),
            (&inp.local_name, &inp.local_email),
        )
        .button(Button::new("identity-local-save").primary().compact().label("Save override").on_click(move |_, _, cx| {
            save.update(cx, |this, cx| this.save_local_identity(save_repo.clone(), cx))
        }))
        .button(
            Button::new("identity-local-clear")
                .ghost()
                .compact()
                .disabled(self.identity.local.is_empty())
                .label("Remove override")
                .on_click(move |_, _, cx| clear.update(cx, |this, cx| this.clear_local_identity(clear_repo.clone(), cx))),
        )
        .status(self.identity.message);
        div()
            .flex()
            .flex_col()
            .gap(px(14.0))
            .child(block)
            .children(self.identity.effective.map(|e| {
                div().text_size(px(12.0)).text_color(theme.text_muted).child(effective_line(&e))
            }))
    }
}

/// "In effect here: Name <email> (repository override)".
fn effective_line(e: &EffectiveIdentity) -> String {
    let who = match (e.identity.name.as_str(), e.identity.email.as_str()) {
        ("", "") => "no identity configured".to_string(),
        (n, "") => n.to_string(),
        ("", m) => format!("<{m}>"),
        (n, m) => format!("{n} <{m}>"),
    };
    let from = match e.source {
        IdentitySource::Repository => "repository override",
        IdentitySource::Global => "global",
        IdentitySource::Unset => "unset",
    };
    format!("In effect here: {who} ({from})")
}
