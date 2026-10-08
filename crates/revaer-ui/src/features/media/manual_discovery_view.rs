//! Manual admission from a confirmed, active association.

use std::{cell::Cell, rc::Rc};

use revaer_api_models::{MediaDiscoveryPreviewResponse, MediaDiscoveryRunResponse};
use uuid::Uuid;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::{platform::spawn_local, prelude::*};

use super::{
    api::{preview_manual_discovery, run_manual_discovery},
    manual_discovery::request,
};
use crate::app::api::ApiCtx;

#[derive(Properties, PartialEq)]
pub(crate) struct ManualDiscoveryProps {
    pub association: Uuid,
}

#[function_component(ManualDiscovery)]
pub(crate) fn manual_discovery(props: &ManualDiscoveryProps) -> Html {
    let api = use_context::<ApiCtx>();
    let candidates = use_state(String::new);
    let preview = use_state(|| None::<MediaDiscoveryPreviewResponse>);
    let outcome = use_state(|| None::<MediaDiscoveryRunResponse>);
    let confirmed = use_state(|| false);
    let busy = use_state(|| false);
    let error = use_state(|| None::<&'static str>);
    let request_generation = use_mut_ref(|| Rc::new(Cell::new(true)));
    {
        let request_generation = request_generation.clone();
        let preview = preview.clone();
        let outcome = outcome.clone();
        let confirmed = confirmed.clone();
        let busy = busy.clone();
        use_effect_with((api.clone(), props.association), move |_| {
            request_generation.borrow().set(false);
            *request_generation.borrow_mut() = Rc::new(Cell::new(true));
            preview.set(None);
            outcome.set(None);
            confirmed.set(false);
            busy.set(false);
            let active = request_generation.borrow().clone();
            move || active.set(false)
        });
    }
    let on_candidates = {
        let candidates = candidates.clone();
        let preview = preview.clone();
        let outcome = outcome.clone();
        let error = error.clone();
        let confirmed = confirmed.clone();
        Callback::from(move |event: InputEvent| {
            candidates.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
            preview.set(None);
            outcome.set(None);
            error.set(None);
            confirmed.set(false);
        })
    };
    let on_confirm = {
        let confirmed = confirmed.clone();
        Callback::from(move |event: Event| {
            confirmed.set(event.target_unchecked_into::<HtmlInputElement>().checked());
        })
    };
    let submit = |queue: bool| {
        let api = api.clone();
        let candidates = candidates.clone();
        let preview = preview.clone();
        let outcome = outcome.clone();
        let error = error.clone();
        let busy = busy.clone();
        let confirmed = confirmed.clone();
        let request_generation = request_generation.clone();
        let association = props.association;
        Callback::from(move |_| {
            if *busy || (queue && (!*confirmed || preview.is_none() || outcome.is_some())) {
                return;
            }
            let input = match request(association, &candidates) {
                Ok(input) => input,
                Err(message) => {
                    error.set(Some(message));
                    return;
                }
            };
            let Some(api) = api.clone() else {
                error.set(Some("Authentication required for discovery."));
                return;
            };
            busy.set(true);
            error.set(None);
            confirmed.set(false);
            if !queue {
                preview.set(None);
            }
            let busy = busy.clone();
            let error = error.clone();
            let preview = preview.clone();
            let outcome = outcome.clone();
            let active = request_generation.borrow().clone();
            spawn_local(async move {
                if queue {
                    let result = run_manual_discovery(&api.client, &input).await;
                    if active.get() {
                        match result {
                            Ok(response) => outcome.set(Some(response)),
                            Err(message) => error.set(Some(message)),
                        }
                    }
                } else {
                    let result = preview_manual_discovery(&api.client, &input).await;
                    if active.get() {
                        match result {
                            Ok(response) => preview.set(Some(response)),
                            Err(message) => error.set(Some(message)),
                        }
                    }
                }
                if active.get() {
                    busy.set(false);
                }
            });
        })
    };
    let has_accepted = preview
        .as_ref()
        .is_some_and(|response| response.previews.iter().any(|row| row.accepted));
    html! {
        <section class="border-b border-base-300 py-4 space-y-3" data-testid="media-manual-discovery" aria-busy={busy.to_string()}>
            <h2 class="text-lg font-semibold">{"Manual discovery"}</h2>
            <label class="form-control"><span>{"Relative candidates"}</span>
                <textarea class="textarea textarea-bordered w-full" rows="4" value={(*candidates).clone()} oninput={on_candidates} disabled={*busy} />
            </label>
            <button type="button" class="btn btn-sm" onclick={submit(false)} disabled={*busy}>{"Preview candidates"}</button>
            {preview.as_ref().map(render_preview).unwrap_or_default()}
            <label class="flex items-center gap-2"><input type="checkbox" class="checkbox checkbox-sm" onchange={on_confirm} checked={*confirmed} disabled={*busy || !has_accepted || outcome.is_some()} />
                <span>{"Queue jobs using the active profile"}</span>
            </label>
            <button type="button" class="btn btn-primary btn-sm" onclick={submit(true)} disabled={*busy || !has_accepted || !*confirmed || outcome.is_some()}>{"Queue jobs"}</button>
            {error.map(|message| html! { <p role="alert">{message}</p> }).unwrap_or_default()}
            {outcome.as_ref().map(render_outcome).unwrap_or_default()}
        </section>
    }
}

fn render_preview(response: &MediaDiscoveryPreviewResponse) -> Html {
    html! {
        <ul class="space-y-1">{for response.previews.iter().enumerate().map(|(index, row)| html! {
            <li key={index} class="break-all">{format!("{} | {} | {}", row.source_path,
                if row.accepted { "Accepted" } else { "Rejected" }, if row.dry_run { "Dry-run" } else { "Execution enabled" })}</li>
        })}</ul>
    }
}

fn render_outcome(response: &MediaDiscoveryRunResponse) -> Html {
    html! {
        <>
            <p role="status">{format!("{} queued; {} skipped.", response.queued_jobs.len(), response.skipped.len())}</p>
            <ul>{for response.queued_jobs.iter().map(|row| html! { <li class="break-all" key={row.media_job_public_id.to_string()}>{format!("{} | {} | {}", row.source_path, row.media_job_public_id, if row.dry_run { "Dry-run" } else { "Execution enabled" })}</li> })}</ul>
            <ul>{for response.skipped.iter().enumerate().map(|(index, row)| html! { <li class="break-all" key={index}>{format!("{} | Skipped", row.source_path)}</li> })}</ul>
        </>
    }
}
