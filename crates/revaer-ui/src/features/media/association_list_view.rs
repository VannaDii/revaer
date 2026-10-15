//! Persisted association selection with bounded pages and fresh-context evidence.

use std::{cell::Cell, rc::Rc};

use uuid::Uuid;
use web_sys::HtmlSelectElement;
use yew::{platform::spawn_local, prelude::*};

use super::{
    api::fetch_association_page, manual_discovery::AssociationPage,
    manual_discovery_view::ManualDiscovery,
};
use crate::{
    app::api::ApiCtx,
    components::atoms::{
        IconButton,
        icons::{IconArrowUp, IconRefreshCw},
    },
};

#[derive(Properties, PartialEq)]
pub(crate) struct AssociationSelectorProps {
    pub created: Option<Uuid>,
}

#[derive(Clone, PartialEq, Eq)]
enum Associations {
    Loading,
    Failed(&'static str),
    Ready(AssociationPage),
}

#[function_component(AssociationSelector)]
pub(crate) fn association_selector(props: &AssociationSelectorProps) -> Html {
    let api = use_context::<ApiCtx>();
    let state = use_state(|| Associations::Loading);
    let cursor = use_state(|| None::<String>);
    let history = use_state(Vec::<Option<String>>::new);
    let reload = use_state(|| false);
    let selected = use_state(|| None::<Uuid>);
    {
        let state = state.clone();
        let selected = selected.clone();
        use_effect_with(
            (api, (*cursor).clone(), *reload, props.created),
            move |(api, cursor, _, created)| {
                state.set(Associations::Loading);
                selected.set(None);
                let active = Rc::new(Cell::new(true));
                if let Some(api) = api.clone() {
                    let active = active.clone();
                    let cursor = cursor.clone();
                    let created = *created;
                    spawn_local(async move {
                        let result = fetch_association_page(&api.client, cursor.as_deref()).await;
                        if active.get() {
                            match result {
                                Ok(page) => {
                                    selected.set(created.filter(|id| {
                                        page.choices
                                            .iter()
                                            .any(|choice| choice.id == *id && choice.manual_ready)
                                    }));
                                    state.set(Associations::Ready(page));
                                }
                                Err(message) => state.set(Associations::Failed(message)),
                            }
                        }
                    });
                } else {
                    state.set(Associations::Failed(
                        "Authentication required to load associations.",
                    ));
                }
                move || active.set(false)
            },
        );
    }
    let next = match &*state {
        Associations::Ready(page) => page.next_cursor.clone(),
        _ => None,
    };
    let on_reload = {
        let reload = reload.clone();
        let cursor = cursor.clone();
        let history = history.clone();
        Callback::from(move |_| {
            cursor.set(None);
            history.set(Vec::new());
            reload.set(!*reload);
        })
    };
    let on_next = {
        let cursor = cursor.clone();
        let history = history.clone();
        let next = next.clone();
        Callback::from(move |_| {
            let mut pages = (*history).clone();
            pages.push((*cursor).clone());
            history.set(pages);
            cursor.set(next.clone());
        })
    };
    let on_previous = {
        let cursor = cursor.clone();
        let history = history.clone();
        Callback::from(move |_| {
            let mut pages = (*history).clone();
            if let Some(page) = pages.pop() {
                cursor.set(page);
                history.set(pages);
            }
        })
    };
    let on_select = {
        let selected = selected.clone();
        let state = state.clone();
        Callback::from(move |event: Event| {
            let value = event.target_unchecked_into::<HtmlSelectElement>().value();
            selected.set(match &*state {
                Associations::Ready(page) => page
                    .choices
                    .iter()
                    .find(|choice| choice.id.to_string() == value && choice.manual_ready)
                    .map(|choice| choice.id),
                _ => None,
            });
        })
    };
    let loading = matches!(*state, Associations::Loading);
    html! {
        <>
            <section class="border-b border-base-300 py-4 space-y-3" data-testid="media-association-list" aria-busy={loading.to_string()}>
                <h2 class="text-lg font-semibold">{"Saved associations"}</h2>
                <div class="flex flex-wrap items-center gap-2">
                    <span title="Reload associations"><IconButton label="Reload associations" icon={html! { <IconRefreshCw size="4" /> }} onclick={on_reload} disabled={loading} /></span>
                    <span title="Previous associations"><IconButton label="Previous associations" icon={html! { <span style="display:inline-flex;transform:rotate(-90deg)"><IconArrowUp size="4" /></span> }} onclick={on_previous} disabled={loading || history.is_empty()} /></span>
                    <span title="Next associations"><IconButton label="Next associations" icon={html! { <span style="display:inline-flex;transform:rotate(90deg)"><IconArrowUp size="4" /></span> }} onclick={on_next} disabled={loading || next.is_none()} /></span>
                </div>
                {match &*state {
                    Associations::Loading => html! { <p role="status">{"Loading associations..."}</p> },
                    Associations::Failed(message) => html! { <p role="alert">{*message}</p> },
                    Associations::Ready(page) => html! {
                        <>
                        <label class="form-control" style="min-width:0"><span>{"Manual association"}</span>
                            <select class="select select-bordered w-full" style="min-width:0" onchange={on_select} value={selected.map_or_else(String::new, |id| id.to_string())}>
                                <option value="" selected={selected.is_none()}>{"Select an association"}</option>
                                {for page.choices.iter().map(|choice| html! {
                                    <option value={choice.id.to_string()} selected={*selected == Some(choice.id)} disabled={!choice.manual_ready}>
                                        {format!("{} | {}:{} | profile v{}{}", choice.key, choice.source_key, if choice.prefix.is_empty() { "Whole root" } else { &choice.prefix }, choice.profile_version, if choice.manual_ready { "" } else { " | Unavailable" })}
                                    </option>
                                })}
                            </select>
                        </label>
                        <super::schedule_view::ScheduleSelector choices={page.choices.clone()} />
                        </>
                    },
                }}
            </section>
            {selected.map(|association| html! { <ManualDiscovery key={association.to_string()} {association} /> }).unwrap_or_default()}
        </>
    }
}
