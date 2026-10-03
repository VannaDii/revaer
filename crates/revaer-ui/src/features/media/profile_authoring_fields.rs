//! Field controls for complete profile authoring.

use web_sys::HtmlInputElement;
use yew::prelude::*;

use super::profile_authoring::ProfileDraft;

pub(super) fn fields(draft: &UseStateHandle<ProfileDraft>, key_locked: bool) -> Html {
    html! {
        <div class="grid gap-3 md:grid-cols-2">
            <fieldset disabled={key_locked} style="min-width:0">{text(draft, "Profile key", &draft.key, 64, false, |d, v| d.key = v)}</fieldset>
            {text(draft, "Display name", &draft.name, 128, false, |d, v| d.name = v)}
            {text(draft, "Description", &draft.description, 1024, false, |d, v| d.description = v)}
            {text(draft, "Desired target key", &draft.target_key, 64, false, |d, v| d.target_key = v)}
            {text(draft, "Desired target version", &draft.target_version, 10, true, |d, v| d.target_version = v)}
            {text(draft, "Policy key", &draft.policy_key, 64, false, |d, v| d.policy_key = v)}
            {text(draft, "Policy version", &draft.policy_version, 10, true, |d, v| d.policy_version = v)}
            {toggle(draft, "Enabled", draft.enabled, |d, v| d.enabled = v)}
            {toggle(draft, "Dry-run only", draft.dry_run_only, |d, v| d.dry_run_only = v)}
        </div>
    }
}

fn text(
    draft: &UseStateHandle<ProfileDraft>,
    label: &'static str,
    value: &str,
    bound: usize,
    numeric: bool,
    update: fn(&mut ProfileDraft, String),
) -> Html {
    let oninput = {
        let draft = draft.clone();
        Callback::from(move |event: InputEvent| {
            let mut next = (*draft).clone();
            update(
                &mut next,
                event.target_unchecked_into::<HtmlInputElement>().value(),
            );
            draft.set(next);
        })
    };
    html! {
        <label class="form-control" style="min-width:0">
            <span>{label}</span>
            <input class="input input-bordered" style="width:100%;min-width:0" type={if numeric {"number"} else {"text"}}
                min={numeric.then_some("1")} max={numeric.then_some("2147483647")}
                maxlength={bound.to_string()} value={value.to_owned()} {oninput} />
        </label>
    }
}

fn toggle(
    draft: &UseStateHandle<ProfileDraft>,
    label: &'static str,
    checked: bool,
    update: fn(&mut ProfileDraft, bool),
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
    html! { <label class="flex items-center gap-2"><input type="checkbox" class="checkbox" {checked} {onchange}/><span>{label}</span></label> }
}
