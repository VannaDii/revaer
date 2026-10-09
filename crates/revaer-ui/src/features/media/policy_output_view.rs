//! Output-policy authoring without conflating policy and profile restrictions.

use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct PolicyOutputDraft {
    dry_run: bool,
    replacement_mode: String,
    quarantine_enabled: bool,
    preservation: PreservationDraft,
}

#[derive(Clone, PartialEq, Eq)]
struct PreservationDraft {
    permissions: bool,
    ownership: bool,
}

impl Default for PolicyOutputDraft {
    fn default() -> Self {
        Self {
            dry_run: true,
            replacement_mode: "disabled".into(),
            quarantine_enabled: true,
            preservation: PreservationDraft {
                permissions: true,
                ownership: true,
            },
        }
    }
}

impl PolicyOutputDraft {
    pub(super) fn request(&self) -> crate::models::MediaPolicyOutput {
        crate::models::MediaPolicyOutput {
            dry_run: self.dry_run,
            replacement_mode: self.replacement_mode.clone(),
            quarantine_enabled: self.quarantine_enabled,
            preservation: crate::models::MediaOutputPreservation {
                preserve_permissions: self.preservation.permissions,
                preserve_ownership: self.preservation.ownership,
            },
        }
    }
}

pub(super) fn fields(draft: &UseStateHandle<PolicyOutputDraft>) -> Html {
    let onchange = {
        let draft = draft.clone();
        Callback::from(move |event: Event| {
            let mut next = (*draft).clone();
            next.replacement_mode = event.target_unchecked_into::<HtmlSelectElement>().value();
            draft.set(next);
        })
    };
    html! {
        <>
            {toggle(draft, "Policy dry-run", draft.dry_run, |d, v| d.dry_run = v)}
            <label class="form-control" style="min-width:0">
                <span>{"Replacement mode"}</span>
                <select class="select select-bordered select-sm" aria-label="Replacement mode" value={draft.replacement_mode.clone()} {onchange}>
                    <option value="disabled" selected={draft.replacement_mode == "disabled"}>{"Disabled"}</option>
                    <option value="atomic_replace" selected={draft.replacement_mode == "atomic_replace"}>{"Atomic replacement"}</option>
                </select>
            </label>
            {toggle(draft, "Quarantine failures", draft.quarantine_enabled, |d, v| d.quarantine_enabled = v)}
            {toggle(draft, "Preserve permissions", draft.preservation.permissions, |d, v| d.preservation.permissions = v)}
            {toggle(draft, "Preserve ownership", draft.preservation.ownership, |d, v| d.preservation.ownership = v)}
        </>
    }
}

fn toggle(
    draft: &UseStateHandle<PolicyOutputDraft>,
    label: &'static str,
    checked: bool,
    update: fn(&mut PolicyOutputDraft, bool),
) -> Html {
    let onchange = {
        let draft = draft.clone();
        Callback::from(move |event: Event| {
            let mut next = (*draft).clone();
            update(
                &mut next,
                event.target_unchecked_into::<HtmlInputElement>().checked(),
            );
            draft.set(next);
        })
    };
    html! { <label class="label cursor-pointer gap-2 justify-start"><input type="checkbox" class="checkbox checkbox-sm" {checked} {onchange}/><span class="label-text">{label}</span></label> }
}
