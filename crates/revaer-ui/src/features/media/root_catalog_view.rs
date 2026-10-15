//! Catalog administration owns physical paths; authoring receives logical choices only.

use std::{cell::Cell, rc::Rc};

use yew::{platform::spawn_local, prelude::*};

use super::{
    api::fetch_root_catalog, association::SourceChoice, association_view::AssociationEditor,
    profile_roots::RootChoice, profile_roots_view::ProfileRootEditor, root_readiness::RootSummary,
};
use crate::{
    app::api::ApiCtx,
    components::atoms::{IconButton, icons::IconRefreshCw},
};

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CatalogDisplay {
    pub summary: RootSummary,
    pub slots: Vec<SlotDisplay>,
    pub sources: Vec<SourceChoice>,
    pub profile_roots: Vec<RootChoice>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SlotDisplay {
    pub key: String,
    pub requested_path: String,
    pub canonical_path: String,
    pub kinds: Vec<KindDisplay>,
    pub evidence: Vec<(&'static str, String)>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct KindDisplay {
    pub name: &'static str,
    pub binding_ready: bool,
    pub binding_reason: Option<String>,
    pub destructive_ready: bool,
    pub destructive_reason: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
enum Catalog {
    Loading,
    Failed(&'static str),
    Ready(CatalogDisplay),
}

#[function_component(RootConfiguration)]
pub(crate) fn root_configuration() -> Html {
    let api = use_context::<ApiCtx>();
    let state = use_state(|| Catalog::Loading);
    let revision = use_state(|| false);
    let profile_revision = use_state(|| false);
    let selected_profile = use_state(|| None::<(uuid::Uuid, bool)>);
    let selection_error = use_state(|| None::<&'static str>);
    let on_edit = {
        let selected_profile = selected_profile.clone();
        let selection_error = selection_error.clone();
        Callback::from(move |id| {
            let confirmation = web_sys::window()
                .ok_or("Profile editor confirmation is unavailable.")
                .and_then(|window| {
                    window
                        .confirm_with_message(
                            "Discard the current profile draft and load this profile for editing?",
                        )
                        .map_err(|_| "Profile editor confirmation failed. The draft is preserved.")
                });
            match confirmation {
                Ok(true) => {
                    let revision = selected_profile
                        .as_ref()
                        .is_none_or(|(_, revision)| !revision);
                    selected_profile.set(Some((id, revision)));
                    selection_error.set(None);
                }
                Ok(false) => {}
                Err(message) => selection_error.set(Some(message)),
            }
        })
    };
    let on_new = {
        let selected_profile = selected_profile.clone();
        Callback::from(move |()| selected_profile.set(None))
    };
    let on_saved = {
        let profile_revision = profile_revision.clone();
        Callback::from(move |()| profile_revision.set(!*profile_revision))
    };
    {
        let state = state.clone();
        use_effect_with((api, *revision), move |(api, _)| {
            state.set(Catalog::Loading);
            let active = Rc::new(Cell::new(true));
            if let Some(api) = api.clone() {
                let active = active.clone();
                spawn_local(async move {
                    let result = fetch_root_catalog(&api.client).await;
                    if active.get() {
                        state.set(match result {
                            Ok(display) => Catalog::Ready(display),
                            Err(message) => Catalog::Failed(message),
                        });
                    }
                });
            } else {
                state.set(Catalog::Failed(
                    "Authentication required to read the root catalog.",
                ));
            }
            move || active.set(false)
        });
    }
    let on_refresh = Callback::from(move |_| revision.set(!*revision));
    let loading = matches!(*state, Catalog::Loading);
    let sources = match &*state {
        Catalog::Ready(display) => display.sources.clone(),
        _ => Vec::new(),
    };
    let choices = match &*state {
        Catalog::Ready(display) => display.profile_roots.clone(),
        _ => Vec::new(),
    };
    html! {
        <>
            <section class="border-b border-base-300 py-4 space-y-3" data-testid="media-root-catalog" aria-busy={loading.to_string()}>
                <div class="flex items-center justify-between gap-2">
                    <h2 class="text-lg font-semibold">{"Root catalog"}</h2>
                    <span title="Reload root catalog"><IconButton label="Reload root catalog" icon={html! { <IconRefreshCw size="4" /> }} onclick={on_refresh} disabled={loading} /></span>
                </div>
                {match &*state {
                    Catalog::Loading => html! { <p role="status">{"Loading root catalog..."}</p> },
                    Catalog::Failed(message) => html! { <p role="alert">{*message}</p> },
                    Catalog::Ready(display) => render_catalog(display),
                }}
            </section>
            {selection_error.map(|message| html! { <p role="alert">{message}</p> }).unwrap_or_default()}
            <ProfileRootEditor key={selected_profile.map_or_else(|| "new".to_owned(), |(id, revision)| format!("{id}-{revision}"))}
                selected={selected_profile.map(|(id, _)| id)} {choices} {on_saved} {on_new} />
            <super::profile_list_view::ProfileList revision={*profile_revision} {on_edit} />
            <AssociationEditor {sources} profile_revision={*profile_revision} />
        </>
    }
}

fn render_catalog(display: &CatalogDisplay) -> Html {
    html! {
        <>
            <dl class="grid gap-2 text-sm sm:grid-cols-3">
                <div><dt>{"Source"}</dt><dd>{display.summary.source}</dd></div>
                <div><dt>{"Attestation"}</dt><dd>{display.summary.attestation}</dd></div>
                <div><dt>{"Generation"}</dt><dd>{display.summary.generation.map_or_else(|| "None".to_owned(), |value| value.to_string())}</dd></div>
            </dl>
            {display.summary.reason.map(|reason| html! { <p role="status">{reason}</p> }).unwrap_or_default()}
            {if display.slots.is_empty() { html! { <p role="status">{"No root slots in the catalog."}</p> } } else { Html::default() }}
            <div class="divide-y divide-base-300">{for display.slots.iter().map(render_slot)}</div>
        </>
    }
}

fn render_slot(slot: &SlotDisplay) -> Html {
    html! {
        <details class="py-3" key={slot.key.clone()}>
            <summary class="cursor-pointer font-medium break-words">{slot.key.clone()}</summary>
            <dl class="grid gap-2 py-2 text-sm md:grid-cols-2">
                <div><dt>{"Requested path"}</dt><dd class="break-all">{slot.requested_path.clone()}</dd></div>
                <div><dt>{"Canonical path"}</dt><dd class="break-all">{slot.canonical_path.clone()}</dd></div>
                {for slot.evidence.iter().map(|(label, value)| html! {
                    <div><dt>{*label}</dt><dd class="break-all">{value.clone()}</dd></div>
                })}
            </dl>
            <ul class="space-y-2 text-sm">{for slot.kinds.iter().map(|kind| html! {
                <li>
                    <strong>{kind.name}</strong>
                    <div>{format!("Binding ready: {}. Destructive ready: {}.", kind.binding_ready, kind.destructive_ready)}</div>
                    {kind.binding_reason.as_ref().map(|reason| html! { <p class="break-words">{reason}</p> }).unwrap_or_default()}
                    {kind.destructive_reason.as_ref().map(|reason| html! { <p class="break-words">{reason}</p> }).unwrap_or_default()}
                </li>
            })}</ul>
        </details>
    }
}
