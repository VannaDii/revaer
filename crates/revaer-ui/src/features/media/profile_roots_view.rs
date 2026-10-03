//! Root-binding fields for the approved immutable profile editor.

use std::{cell::Cell, rc::Rc};
use web_sys::HtmlSelectElement;
use yew::{platform::spawn_local, prelude::*};

use super::profile_roots::{ProfileRootDraft, ProfileRootKind, RootChoice, SelectionReadiness};
use super::{
    api::{create_profile_version, fetch_profile_edit, replace_profile_version},
    profile_authoring::{ProfileDraft, ProfileEdit},
    profile_authoring_fields,
};
use crate::app::api::ApiCtx;

#[derive(Properties, PartialEq)]
pub(crate) struct ProfileRootEditorProps {
    pub choices: Vec<RootChoice>,
    pub on_saved: Callback<()>,
    pub selected: Option<uuid::Uuid>,
    pub on_new: Callback<()>,
}

#[function_component(ProfileRootEditor)]
pub(crate) fn profile_root_editor(props: &ProfileRootEditorProps) -> Html {
    let draft = use_state(ProfileRootDraft::default);
    let error = use_state(|| None::<&'static str>);
    let profile = use_state(ProfileDraft::default);
    let busy = use_state(|| false);
    let saved = use_state(|| None::<i32>);
    let edit = use_state(|| None::<ProfileEdit>);
    let request_scope = use_mut_ref(|| Rc::new(Cell::new(true)));
    let api = use_context::<ApiCtx>();
    {
        let busy = busy.clone();
        let error = error.clone();
        let draft = draft.clone();
        let profile = profile.clone();
        let edit = edit.clone();
        let request_scope = request_scope.clone();
        let load_allowed = profile.key.is_empty() && edit.is_none();
        let saved = saved.clone();
        use_effect_with((api.clone(), props.selected), move |(api, selected)| {
            let active = Rc::new(Cell::new(true));
            request_scope.borrow().set(false);
            *request_scope.borrow_mut() = active.clone();
            if let Some(id) = *selected {
                busy.set(true);
                edit.set(None);
                if !load_allowed {
                    error.set(Some("Authentication changed. The draft is preserved; select the profile again to review its current version."));
                    saved.set(None);
                    busy.set(false);
                } else if let Some(api) = api.clone() {
                    let active = active.clone();
                    spawn_local(async move {
                        let result = fetch_profile_edit(&api.client, id).await;
                        if active.get() {
                            match result {
                                Ok(loaded) => {
                                    profile.set(ProfileDraft::from_request(&loaded.profile));
                                    draft.set(ProfileRootDraft::from_request(&loaded.profile));
                                    edit.set(Some(loaded));
                                    error.set(None);
                                }
                                Err(message) => error.set(Some(message)),
                            }
                            busy.set(false);
                        }
                    });
                } else {
                    error.set(Some(
                        "Authentication required to load the selected profile.",
                    ));
                    busy.set(false);
                }
            }
            move || active.set(false)
        });
    }
    let onsubmit = {
        let draft = draft.clone();
        let profile = profile.clone();
        let choices = props.choices.clone();
        let busy = busy.clone();
        let saved = saved.clone();
        let error = error.clone();
        let on_saved = props.on_saved.clone();
        let edit = edit.clone();
        let selected = props.selected;
        Callback::from(move |event: SubmitEvent| {
            event.prevent_default();
            if *busy || saved.is_some() || (selected.is_some() && edit.is_none()) {
                return;
            }
            let request = match profile.request(&draft, &choices) {
                Ok(request) => request,
                Err(message) => {
                    error.set(Some(message));
                    return;
                }
            };
            let Some(api) = api.clone() else {
                error.set(Some("Authentication required to save a profile."));
                return;
            };
            busy.set(true);
            error.set(None);
            let busy = busy.clone();
            let saved = saved.clone();
            let error = error.clone();
            let on_saved = on_saved.clone();
            let edit = (*edit).clone();
            let active = request_scope.borrow().clone();
            spawn_local(async move {
                let result = match edit {
                    Some(edit) => replace_profile_version(&api.client, &edit, &request).await,
                    None => create_profile_version(&api.client, &request)
                        .await
                        .map(|()| 1),
                };
                if !active.get() {
                    return;
                }
                match result {
                    Ok(version) => {
                        saved.set(Some(version));
                        on_saved.emit(());
                    }
                    Err(message) => error.set(Some(message)),
                }
                busy.set(false);
            });
        })
    };
    let on_new = {
        let profile = profile.clone();
        let draft = draft.clone();
        let saved = saved.clone();
        let error = error.clone();
        let edit = edit.clone();
        let on_new = props.on_new.clone();
        let selected = props.selected;
        Callback::from(move |_| {
            if selected.is_some() && saved.is_none() {
                match web_sys::window()
                    .ok_or("Profile editor confirmation is unavailable.")
                    .and_then(|window| {
                        window
                            .confirm_with_message(
                                "Discard the current profile draft and create a new profile?",
                            )
                            .map_err(
                                |_| "Profile editor confirmation failed. The draft is preserved.",
                            )
                    }) {
                    Ok(true) => {}
                    Ok(false) => return,
                    Err(message) => {
                        error.set(Some(message));
                        return;
                    }
                }
            }
            profile.set(ProfileDraft::default());
            draft.set(ProfileRootDraft::default());
            saved.set(None);
            edit.set(None);
            error.set(None);
            on_new.emit(());
        })
    };
    html! {
        <section class="border-b border-base-300 py-4 space-y-3" data-testid="media-profile-root-editor">
            <h2 class="text-lg font-semibold">{if props.selected.is_some() { "Edit profile" } else { "Create profile" }}</h2>
            {saved.map(|version| html! { <p role="status">{format!("Profile saved as version {version}.")}</p> }).unwrap_or_default()}
            {edit.as_ref().map(|edit| html! { <p>{format!("Current version: {}", edit.version)}</p> }).unwrap_or_default()}
            {error.map(|message| html! { <p role="alert">{message}</p> }).unwrap_or_default()}
            <form {onsubmit}>
            <fieldset disabled={*busy || saved.is_some() || (props.selected.is_some() && edit.is_none())} class="space-y-3" style="min-width:0">
                {profile_authoring_fields::fields(&profile, props.selected.is_some())}
                <div class="grid gap-3 md:grid-cols-2">
                {for ProfileRootKind::ALL.map(|kind| render_field(kind, &draft, &error, &props.choices))}
                </div>
                <button type="submit" class="btn btn-primary" disabled={*busy || saved.is_some()}>{if *busy { "Saving..." } else if props.selected.is_some() { "Save profile" } else { "Create profile" }}</button>
            </fieldset>
            </form>
            {if saved.is_some() || props.selected.is_some() { html! { <button type="button" class="btn" disabled={*busy} onclick={on_new}>{"New profile"}</button> } } else { Html::default() }}
        </section>
    }
}

fn render_field(
    kind: ProfileRootKind,
    draft: &UseStateHandle<ProfileRootDraft>,
    error: &UseStateHandle<Option<&'static str>>,
    choices: &[RootChoice],
) -> Html {
    let selected = draft.selected(kind);
    let readiness = draft.readiness(kind, choices);
    let onchange = {
        let draft = draft.clone();
        let error = error.clone();
        let choices = choices.to_vec();
        Callback::from(move |event: Event| {
            let key = event.target_unchecked_into::<HtmlSelectElement>().value();
            let mut updated = (*draft).clone();
            match updated.select(kind, &key, &choices) {
                Ok(()) => {
                    draft.set(updated);
                    error.set(None);
                }
                Err(message) => error.set(Some(message)),
            }
        })
    };
    html! {
        <div style="min-width:0" class="space-y-1">
            <label class="form-control" style="min-width:0">
                <span>{kind.label()}</span>
                <select class="select select-bordered" style="width:100%;min-width:0" value={selected.to_owned()} {onchange}>
                    <option value="" selected={selected.is_empty()}>{if kind.required() { "Select a root" } else { "None" }}</option>
                    {if readiness == SelectionReadiness::Unavailable {
                        html! { <option value={selected.to_owned()} disabled=true selected=true>{format!("{selected} (unavailable)")}</option> }
                    } else { Html::default() }}
                    {for choices.iter().filter(|choice| choice.kind == kind.kind()).map(|choice| html! {
                        <option value={choice.key.clone()} selected={selected == choice.key} disabled={!choice.binding_ready}>{choice.key.clone()}</option>
                    })}
                </select>
            </label>
            <p class="text-sm" role="status">{readiness.message()}</p>
        </div>
    }
}
