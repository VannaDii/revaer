//! Explicit source-association authoring against the immutable configuration API.

use std::{cell::Cell, rc::Rc};

use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::{platform::spawn_local, prelude::*};

use super::{
    api::{create_association, fetch_profile_choices},
    association::{AssociationDraft, ProfileChoices, SourceChoice, SourceScope},
};
use crate::{
    app::api::ApiCtx,
    components::atoms::{
        IconButton,
        icons::{IconArrowUp, IconRefreshCw},
    },
};

#[derive(Properties, PartialEq)]
pub(crate) struct AssociationEditorProps {
    pub sources: Vec<SourceChoice>,
    pub profile_revision: bool,
}

#[derive(Clone, PartialEq, Eq)]
enum Profiles {
    Loading,
    Failed(&'static str),
    Ready(ProfileChoices),
}

#[function_component(AssociationEditor)]
pub(crate) fn association_editor(props: &AssociationEditorProps) -> Html {
    let api = use_context::<ApiCtx>();
    let draft = use_state(AssociationDraft::default);
    let profiles = use_state(|| Profiles::Loading);
    let cursor = use_state(|| None::<String>);
    let history = use_state(Vec::<Option<String>>::new);
    let reload = use_state(|| false);
    let busy = use_state(|| false);
    let error = use_state(|| None::<&'static str>);
    let success = use_state(|| None::<String>);
    let manual_association = use_state(|| None::<uuid::Uuid>);
    {
        let error = error.clone();
        use_effect_with((*draft).clone(), move |_| {
            error.set(None);
            || ()
        });
    }
    {
        let profiles = profiles.clone();
        use_effect_with(
            (
                api.clone(),
                (*cursor).clone(),
                *reload,
                props.profile_revision,
            ),
            move |(api, cursor, _, _)| {
                let active = Rc::new(Cell::new(true));
                profiles.set(Profiles::Loading);
                if let Some(api) = api.clone() {
                    let cursor = cursor.clone();
                    let active = active.clone();
                    spawn_local(async move {
                        let result = fetch_profile_choices(&api.client, cursor.as_deref()).await;
                        if active.get() {
                            profiles.set(match result {
                                Ok(page) => Profiles::Ready(page),
                                Err(message) => Profiles::Failed(message),
                            });
                        }
                    });
                } else {
                    profiles.set(Profiles::Failed(
                        "Authentication required to load profile versions.",
                    ));
                }
                move || active.set(false)
            },
        );
    }
    let active_profiles = match &*profiles {
        Profiles::Ready(page) => page.active.clone(),
        _ => Vec::new(),
    };
    let on_profile = {
        let draft = draft.clone();
        let active_profiles = active_profiles.clone();
        Callback::from(move |event: Event| {
            let selected = event.target_unchecked_into::<HtmlSelectElement>().value();
            let mut updated = (*draft).clone();
            updated.profile = active_profiles
                .iter()
                .find(|profile| profile.id.to_string() == selected)
                .map(|profile| (profile.id, profile.version));
            draft.set(updated);
        })
    };
    let on_source = {
        let draft = draft.clone();
        Callback::from(move |event: Event| {
            let mut updated = (*draft).clone();
            updated.source = event.target_unchecked_into::<HtmlSelectElement>().value();
            draft.set(updated);
        })
    };
    let on_reload = {
        let cursor = cursor.clone();
        let history = history.clone();
        Callback::from(move |_| {
            cursor.set(None);
            history.set(Vec::new());
            reload.set(!*reload);
        })
    };
    let on_next = {
        let next = match &*profiles {
            Profiles::Ready(page) => page.next_cursor.clone(),
            _ => None,
        };
        let cursor = cursor.clone();
        let history = history.clone();
        Callback::from(move |_| {
            let mut previous = (*history).clone();
            previous.push((*cursor).clone());
            history.set(previous);
            cursor.set(next.clone());
        })
    };
    let on_previous = {
        let history = history.clone();
        Callback::from(move |_| {
            let mut previous = (*history).clone();
            if let Some(page) = previous.pop() {
                cursor.set(page);
                history.set(previous);
            }
        })
    };
    let on_new = {
        let manual_association = manual_association.clone();
        let draft = draft.clone();
        let success = success.clone();
        let error = error.clone();
        Callback::from(move |_| {
            draft.set(AssociationDraft::default());
            success.set(None);
            manual_association.set(None);
            error.set(None);
        })
    };
    let on_submit = {
        let manual_association = manual_association.clone();
        let draft = draft.clone();
        let sources = props.sources.clone();
        let active_profiles = active_profiles.clone();
        let busy = busy.clone();
        let error = error.clone();
        let success = success.clone();
        Callback::from(move |event: SubmitEvent| {
            event.prevent_default();
            if *busy {
                return;
            }
            success.set(None);
            let request = match draft.request(&active_profiles, &sources) {
                Ok(request) => request,
                Err(message) => {
                    error.set(Some(message));
                    return;
                }
            };
            let Some(api) = api.clone() else {
                error.set(Some("Authentication required to create an association."));
                return;
            };
            busy.set(true);
            error.set(None);
            let busy = busy.clone();
            let error = error.clone();
            let success = success.clone();
            let manual_association = manual_association.clone();
            spawn_local(async move {
                match create_association(&api.client, &request).await {
                    Ok((message, association)) => {
                        success.set(Some(message));
                        manual_association.set(association);
                    }
                    Err(message) => error.set(Some(message)),
                }
                busy.set(false);
            });
        })
    };
    let unavailable = *busy
        || active_profiles.is_empty()
        || !props.sources.iter().any(|source| source.binding_ready);
    let has_next = matches!(&*profiles, Profiles::Ready(page) if page.next_cursor.is_some());
    html! {
        <>
        <section class="border-b border-base-300 py-4 space-y-3" data-testid="media-association-editor">
            <h2 class="text-lg font-semibold">{"Discovery association"}</h2>
            <div class="flex flex-wrap items-center gap-2">
                <span title="Reload profile versions"><IconButton label="Reload profile versions" icon={html! { <IconRefreshCw size="4" /> }} onclick={on_reload} disabled={*busy} /></span>
                <span title="Previous profile page"><IconButton label="Previous profile page" icon={html! { <span style="display:inline-flex;transform:rotate(-90deg)"><IconArrowUp size="4" /></span> }} onclick={on_previous} disabled={*busy || history.is_empty()} /></span>
                <span title="Next profile page"><IconButton label="Next profile page" icon={html! { <span style="display:inline-flex;transform:rotate(90deg)"><IconArrowUp size="4" /></span> }} onclick={on_next} disabled={*busy || !has_next} /></span>
            </div>
            {match &*profiles {
                Profiles::Loading => html! { <p role="status">{"Loading active profile versions..."}</p> },
                Profiles::Failed(message) => html! { <p role="alert">{*message}</p> },
                Profiles::Ready(page) if page.active.is_empty() => html! { <p role="status">{"No active profile versions on this page."}</p> },
                Profiles::Ready(_) => Html::default(),
            }}
            {if props.sources.iter().any(|source| source.binding_ready) { Html::default() } else {
                html! { <p role="status">{"No binding-ready source roots are available."}</p> }
            }}
            <form onsubmit={on_submit} class="space-y-3">
                <fieldset disabled={*busy || success.is_some()} class="grid gap-3 md:grid-cols-2" style="min-width:0">
                    {text_field("Association key", &draft.key, edit_text(&draft, |value, text| value.key = text))}
                    <label class="form-control" style="min-width:0"><span>{"Active profile version"}</span>
                        <select class="select select-bordered" style="width:100%;min-width:0" value={draft.profile.map_or_else(String::new, |(id, _)| id.to_string())} onchange={on_profile}>
                            <option value="" selected={draft.profile.is_none()}>{"Select a profile version"}</option>
                            {for active_profiles.iter().map(|profile| html! {
                                <option value={profile.id.to_string()} selected={draft.profile == Some((profile.id, profile.version))}>{format!("{} - version {}", profile.key, profile.version)}</option>
                            })}
                        </select>
                    </label>
                    <label class="form-control" style="min-width:0"><span>{"Source root"}</span>
                        <select class="select select-bordered" style="width:100%;min-width:0" value={draft.source.clone()} onchange={on_source}>
                            <option value="" selected={draft.source.is_empty()}>{"Select a source root"}</option>
                            {for props.sources.iter().map(|source| html! {
                                <option value={source.key.clone()} selected={draft.source == source.key} disabled={!source.binding_ready}>{source.key.clone()}</option>
                            })}
                        </select>
                    </label>
                    {props.sources.iter().find(|source| source.key == draft.source).map(|source| html! {
                        <p class="text-sm">{format!("Binding ready: {}. Destructive ready: {}.", source.binding_ready, source.destructive_ready)}</p>
                    }).unwrap_or_default()}
                    <fieldset class="space-y-2"><legend>{"Source scope"}</legend>
                        {scope_option(&draft, SourceScope::WholeRoot, "Whole root")}
                        {scope_option(&draft, SourceScope::Prefix, "Relative prefix")}
                    </fieldset>
                    {if draft.scope == SourceScope::Prefix {
                        text_field("Relative prefix", &draft.prefix, edit_text(&draft, |value, text| value.prefix = text))
                    } else { Html::default() }}
                    <fieldset class="flex flex-wrap gap-3"><legend>{"Discovery modes"}</legend>
                        {mode_option(&draft, "Manual", draft.modes.manual_enabled, |value, checked| value.modes.manual_enabled = checked)}
                        {mode_option(&draft, "Watcher", draft.modes.watcher_enabled, |value, checked| value.modes.watcher_enabled = checked)}
                        {mode_option(&draft, "Schedule", draft.modes.schedule_enabled, |value, checked| value.modes.schedule_enabled = checked)}
                    </fieldset>
                </fieldset>
                {error.as_ref().map(|message| html! { <p role="alert">{*message}</p> }).unwrap_or_default()}
                {success.as_ref().map(|message| html! { <p role="status" class="break-words">{message}</p> }).unwrap_or_default()}
                <button class="btn btn-primary" type="submit" disabled={unavailable || success.is_some()}>{"Create association"}</button>
                {if success.is_some() { html! { <button class="btn" type="button" onclick={on_new}>{"New association"}</button> } } else { Html::default() }}
            </form>
        </section>
        <super::association_list_view::AssociationSelector created={*manual_association} />
        </>
    }
}

fn text_field(label: &'static str, value: &str, oninput: Callback<InputEvent>) -> Html {
    html! { <label class="form-control" style="min-width:0"><span>{label}</span><input class="input input-bordered" style="width:100%;min-width:0" value={value.to_owned()} {oninput} /></label> }
}

fn edit_text(
    draft: &UseStateHandle<AssociationDraft>,
    edit: fn(&mut AssociationDraft, String),
) -> Callback<InputEvent> {
    let draft = draft.clone();
    Callback::from(move |event: InputEvent| {
        let mut updated = (*draft).clone();
        edit(
            &mut updated,
            event.target_unchecked_into::<HtmlInputElement>().value(),
        );
        draft.set(updated);
    })
}

fn scope_option(
    draft: &UseStateHandle<AssociationDraft>,
    scope: SourceScope,
    label: &'static str,
) -> Html {
    let selected = draft.scope == scope;
    let draft = draft.clone();
    let onchange = Callback::from(move |_| {
        let mut updated = (*draft).clone();
        updated.scope = scope;
        draft.set(updated);
    });
    html! { <label class="flex items-center gap-2"><input type="radio" class="radio radio-sm" name="association-scope" checked={selected} {onchange} />{label}</label> }
}

fn mode_option(
    draft: &UseStateHandle<AssociationDraft>,
    label: &'static str,
    checked: bool,
    edit: fn(&mut AssociationDraft, bool),
) -> Html {
    let draft = draft.clone();
    let onchange = Callback::from(move |event: Event| {
        let mut updated = (*draft).clone();
        edit(
            &mut updated,
            event.target_unchecked_into::<HtmlInputElement>().checked(),
        );
        draft.set(updated);
    });
    html! { <label class="flex items-center gap-2"><input type="checkbox" class="checkbox checkbox-sm" {checked} {onchange} />{label}</label> }
}
