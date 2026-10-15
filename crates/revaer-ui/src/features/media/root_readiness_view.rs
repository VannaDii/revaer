//! Read-only root readiness in the operator workflow.

use std::{cell::Cell, rc::Rc};

use yew::{platform::spawn_local, prelude::*};

use super::{
    api::fetch_root_readiness,
    root_readiness::{RootLoadFailure, RootSummary},
};
use crate::{
    app::api::ApiCtx,
    components::atoms::{IconButton, icons::IconRefreshCw},
};

#[derive(Clone, PartialEq, Eq)]
enum LoadState {
    Loading,
    Failed(RootLoadFailure),
    Loaded(RootSummary),
}

#[function_component(RootReadiness)]
pub(crate) fn root_readiness() -> Html {
    let api = use_context::<ApiCtx>();
    let state = use_state(|| LoadState::Loading);
    let refresh = use_state(|| false);
    {
        let state = state.clone();
        use_effect_with((api, *refresh), move |(api, _)| {
            let active = Rc::new(Cell::new(true));
            state.set(LoadState::Loading);
            if let Some(api) = api.clone() {
                let active = active.clone();
                spawn_local(async move {
                    let result = fetch_root_readiness(&api.client).await;
                    // A response from an old API context or unmounted view cannot publish.
                    if active.get() {
                        state.set(match result {
                            Ok(summary) => LoadState::Loaded(summary),
                            Err(error) => LoadState::Failed(error),
                        });
                    }
                });
            } else {
                state.set(LoadState::Failed(RootLoadFailure::Authentication));
            }
            move || active.set(false)
        });
    }
    let on_refresh = Callback::from(move |_| refresh.set(!*refresh));
    let loading = matches!(*state, LoadState::Loading);
    html! {
        <section class="border-y border-base-300 py-4 space-y-3"
            aria-labelledby="media-roots-heading" aria-busy={loading.to_string()}
            data-testid="media-root-readiness">
            <div class="flex items-center justify-between gap-2">
                <h2 id="media-roots-heading" class="text-lg font-semibold">{"Root readiness"}</h2>
                <span title="Refresh root readiness">
                    <IconButton label="Refresh root readiness" icon={html! { <IconRefreshCw size="4" /> }}
                        onclick={on_refresh} disabled={loading} />
                </span>
            </div>
            {match &*state {
                LoadState::Loading => html! { <p role="status">{"Loading root readiness..."}</p> },
                LoadState::Failed(error) => html! { <p role="alert">{error.message()}</p> },
                LoadState::Loaded(summary) => render_summary(summary),
            }}
        </section>
    }
}

fn render_summary(summary: &RootSummary) -> Html {
    html! {
        <>
            <dl class="grid grid-cols-1 gap-2 text-sm sm:grid-cols-3">
                <div><dt class="font-medium">{"Catalog source"}</dt><dd>{summary.source}</dd></div>
                <div><dt class="font-medium">{"Attestation"}</dt><dd>{summary.attestation}</dd></div>
                <div><dt class="font-medium">{"Generation"}</dt><dd>{summary.generation.map_or_else(|| "None".to_owned(), |value| value.to_string())}</dd></div>
            </dl>
            {summary.reason.map(|reason| html! {
                <p role="status" class="break-words">{reason}</p>
            }).unwrap_or_default()}
            {if summary.generation.is_some() && summary.kinds.iter().all(|row| row.attested == 0) {
                html! { <p role="status">{"The catalog contains no attested root slots."}</p> }
            } else { Html::default() }}
            <div class="overflow-x-auto">
                <table class="table table-sm" style="width:100%;table-layout:fixed;font-size:12px">
                    <caption class="text-left text-sm font-medium">{"Root slots by kind"}</caption>
                    <colgroup>
                        <col style="width:24%" /><col style="width:23%" />
                        <col style="width:23%" /><col style="width:30%" />
                    </colgroup>
                    <thead><tr>
                        <th scope="col" style="padding:4px;white-space:normal">{"Kind"}</th>
                        <th scope="col" style="padding:4px;white-space:normal">{"Attested"}</th>
                        <th scope="col" style="padding:4px;white-space:normal">{"Binding ready"}</th>
                        <th scope="col" style="padding:4px;white-space:normal">{"Destructive ready"}</th>
                    </tr></thead>
                    <tbody>{for summary.kinds.iter().map(|row| html! {
                        <tr key={row.kind}>
                            <th scope="row" style="padding:4px">{row.kind}</th>
                            <td style="padding:4px">{row.attested}</td>
                            <td style="padding:4px">{row.binding}</td>
                            <td style="padding:4px">{row.destructive}</td>
                        </tr>
                    })}</tbody>
                </table>
            </div>
        </>
    }
}
