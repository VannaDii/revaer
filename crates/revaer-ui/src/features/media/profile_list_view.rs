//! Paginated saved-profile presentation, independent of dashboard diagnostics.

use std::{cell::Cell, rc::Rc};

use yew::{platform::spawn_local, prelude::*};

use super::{
    api::fetch_profile_page,
    profile_list::{ProfilePage, ProfileSummary},
};
use crate::{
    app::api::ApiCtx,
    components::atoms::{
        IconButton,
        icons::{IconArrowUp, IconRefreshCw},
    },
};

#[derive(Properties, PartialEq)]
pub(crate) struct ProfileListProps {
    pub revision: bool,
    pub on_edit: Callback<uuid::Uuid>,
}

#[derive(Clone, PartialEq, Eq)]
enum Profiles {
    Loading,
    Failed(&'static str),
    Ready(ProfilePage),
}

#[function_component(ProfileList)]
pub(crate) fn profile_list(props: &ProfileListProps) -> Html {
    let api = use_context::<ApiCtx>();
    let state = use_state(|| Profiles::Loading);
    let cursor = use_state(|| None::<String>);
    let history = use_state(Vec::<Option<String>>::new);
    let reload = use_state(|| false);
    {
        let state = state.clone();
        use_effect_with(
            (api, (*cursor).clone(), *reload, props.revision),
            move |(api, cursor, _, _)| {
                state.set(Profiles::Loading);
                let active = Rc::new(Cell::new(true));
                if let Some(api) = api.clone() {
                    let active = active.clone();
                    let cursor = cursor.clone();
                    spawn_local(async move {
                        let result = fetch_profile_page(&api.client, cursor.as_deref()).await;
                        if active.get() {
                            state.set(match result {
                                Ok(page) => Profiles::Ready(page),
                                Err(message) => Profiles::Failed(message),
                            });
                        }
                    });
                } else {
                    state.set(Profiles::Failed(
                        "Authentication required to load profiles.",
                    ));
                }
                move || active.set(false)
            },
        );
    }
    let loading = matches!(*state, Profiles::Loading);
    let next_cursor = match &*state {
        Profiles::Ready(page) => page.next_cursor.clone(),
        _ => None,
    };
    let on_next = {
        let cursor = cursor.clone();
        let history = history.clone();
        let next_cursor = next_cursor.clone();
        Callback::from(move |_| {
            if let Some(next) = next_cursor.clone() {
                let mut previous = (*history).clone();
                previous.push((*cursor).clone());
                history.set(previous);
                cursor.set(Some(next));
            }
        })
    };
    let on_previous = {
        let history = history.clone();
        Callback::from(move |_| {
            let mut previous = (*history).clone();
            if let Some(next) = previous.pop() {
                history.set(previous);
                cursor.set(next);
            }
        })
    };
    let on_reload = Callback::from(move |_| reload.set(!*reload));
    html! {
        <section class="media-profile-list border-b border-base-300 py-4 space-y-3" data-testid="media-profile-list" aria-busy={loading.to_string()}>
            <div class="flex items-center justify-between gap-2">
                <h2 class="text-lg font-semibold">{"Profiles"}</h2>
                <span title="Reload profiles"><IconButton label="Reload profiles" icon={html! { <IconRefreshCw size="4" /> }} onclick={on_reload} disabled={loading} /></span>
            </div>
            {match &*state {
                Profiles::Loading => html! { <p role="status">{"Loading profiles..."}</p> },
                Profiles::Failed(message) => html! { <p role="alert">{*message}</p> },
                Profiles::Ready(page) if page.profiles.is_empty() => html! { <p role="status">{"No profiles."}</p> },
                Profiles::Ready(page) => html! { <ul class="divide-y divide-base-300">{for page.profiles.iter().map(|profile| render_profile(profile, &props.on_edit))}</ul> },
            }}
            <div class="flex items-center gap-2">
                <span title="Previous profiles"><IconButton label="Previous profiles" disabled={loading || history.is_empty()}
                    icon={html! { <span class="inline-block -rotate-90"><IconArrowUp size="4" /></span> }} onclick={on_previous} /></span>
                <span title="Next profiles"><IconButton label="Next profiles" disabled={loading || next_cursor.is_none()}
                    icon={html! { <span class="inline-block rotate-90"><IconArrowUp size="4" /></span> }} onclick={on_next} /></span>
            </div>
        </section>
    }
}

fn render_profile(profile: &ProfileSummary, on_edit: &Callback<uuid::Uuid>) -> Html {
    let id = profile.id;
    let on_edit = on_edit.clone();
    let onclick = Callback::from(move |_| on_edit.emit(id));
    html! {
        <li key={profile.id.to_string()} class="py-3 space-y-2" data-testid="media-profile-summary">
            <div class="font-medium break-words">{profile.name.clone()}</div>
            <button type="button" class="btn btn-sm" aria-label={format!("Edit profile {}", profile.key)} {onclick}>{"Edit"}</button>
            <div class="text-sm break-words">{format!("{} | {} | {} | {}", profile.key, profile.lifecycle,
                if profile.enabled { "Enabled" } else { "Disabled" }, if profile.dry_run_only { "Dry-run only" } else { "Policy-controlled" })}</div>
            <dl class="grid gap-2 text-sm sm:grid-cols-2 lg:grid-cols-4">
                <div><dt>{"Latest version"}</dt><dd>{profile.latest}</dd></div>
                <div><dt>{"Active version"}</dt><dd>{profile.active.map_or_else(|| "None".to_owned(), |version| version.to_string())}</dd></div>
                <div><dt>{"Target"}</dt><dd class="break-words">{profile.target.clone()}</dd></div>
                <div><dt>{"Policy"}</dt><dd class="break-words">{profile.policy.clone()}</dd></div>
            </dl>
            {if profile.description.is_empty() { Html::default() } else { html! { <p class="text-sm break-words">{profile.description.clone()}</p> } }}
            <ul class="text-sm space-y-1">{for profile.roots.iter().map(|root| html! {
                <li class="break-words">
                    {format!("{}: {} | binding {} | destructive {}", root.kind, root.key,
                        if root.binding { "ready" } else { "not ready" }, if root.destructive { "ready" } else { "not ready" })}
                    {root.reason.as_ref().map(|reason| html! { <span>{format!(" | {reason}")}</span> }).unwrap_or_default()}
                </li>
            })}</ul>
        </li>
    }
}
