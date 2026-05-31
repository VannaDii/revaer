use crate::app::api::ApiCtx;
use crate::features::media::api::{
    apply_yaml, export_yaml, fetch_jobs, fetch_latest_capability, fetch_profiles, fetch_readiness,
    refresh_capability, validate_yaml,
};
use crate::features::media::state::MediaViewState;
use yew::platform::spawn_local;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub(crate) struct MediaPageProps {
    pub on_success_toast: Callback<String>,
    pub on_error_toast: Callback<String>,
}

#[function_component(MediaPage)]
pub(crate) fn media_page(props: &MediaPageProps) -> Html {
    let api = use_context::<ApiCtx>();
    let state = use_state(MediaViewState::default);
    let busy = use_state(|| false);
    let yaml_input = use_state(String::new);
    let validation_status = use_state(|| None::<String>);

    let on_refresh = {
        let api = api.clone();
        let state = state.clone();
        let busy = busy.clone();
        let on_error_toast = props.on_error_toast.clone();
        Callback::from(move |_| {
            let Some(api) = api.clone() else {
                on_error_toast.emit("Media API context is unavailable".to_string());
                return;
            };
            busy.set(true);
            let state = state.clone();
            let busy = busy.clone();
            let on_error_toast = on_error_toast.clone();
            spawn_local(async move {
                let profiles = fetch_profiles(&api.client).await;
                let jobs = fetch_jobs(&api.client).await;
                let readiness = fetch_readiness(&api.client).await;
                let latest = fetch_latest_capability(&api.client).await;
                match (profiles, jobs, readiness, latest) {
                    (Ok(profiles), Ok(jobs), Ok(readiness), Ok(latest)) => {
                        let current = (*state).clone();
                        state.set(MediaViewState {
                            profiles: profiles.profiles,
                            jobs: jobs.jobs,
                            readiness: Some(readiness),
                            latest_capability: latest.snapshot,
                            yaml_export: current.yaml_export,
                        });
                    }
                    _ => on_error_toast.emit("Failed to refresh media snapshot".to_string()),
                }
                busy.set(false);
            });
        })
    };

    {
        let on_refresh = on_refresh.clone();
        use_effect_with((), move |_| {
            on_refresh.emit(());
            || ()
        });
    }
    let on_refresh_click = {
        let on_refresh = on_refresh.clone();
        Callback::from(move |_: MouseEvent| on_refresh.emit(()))
    };

    let on_refresh_capability = {
        let api = api.clone();
        let on_success_toast = props.on_success_toast.clone();
        let on_error_toast = props.on_error_toast.clone();
        let on_refresh = on_refresh.clone();
        Callback::from(move |_| {
            let Some(api) = api.clone() else {
                on_error_toast.emit("Media API context is unavailable".to_string());
                return;
            };
            let on_success_toast = on_success_toast.clone();
            let on_error_toast = on_error_toast.clone();
            let on_refresh = on_refresh.clone();
            spawn_local(async move {
                match refresh_capability(&api.client).await {
                    Ok(_) => {
                        on_success_toast.emit("Media capability refreshed".to_string());
                        on_refresh.emit(());
                    }
                    Err(error) => on_error_toast.emit(error),
                }
            });
        })
    };

    let on_export = {
        let api = api.clone();
        let state = state.clone();
        let on_error_toast = props.on_error_toast.clone();
        let on_success_toast = props.on_success_toast.clone();
        Callback::from(move |_| {
            let Some(api) = api.clone() else {
                on_error_toast.emit("Media API context is unavailable".to_string());
                return;
            };
            let state = state.clone();
            let on_error_toast = on_error_toast.clone();
            let on_success_toast = on_success_toast.clone();
            spawn_local(async move {
                match export_yaml(&api.client).await {
                    Ok(response) => {
                        let mut next = (*state).clone();
                        next.yaml_export = Some(response.yaml_payload);
                        state.set(next);
                        on_success_toast.emit("Media YAML exported".to_string());
                    }
                    Err(error) => on_error_toast.emit(error),
                }
            });
        })
    };

    let on_yaml_input = {
        let yaml_input = yaml_input.clone();
        Callback::from(move |event: InputEvent| {
            let value = event
                .target_unchecked_into::<web_sys::HtmlTextAreaElement>()
                .value();
            yaml_input.set(value);
        })
    };

    let on_validate = {
        let api = api.clone();
        let yaml_input = yaml_input.clone();
        let validation_status = validation_status.clone();
        let on_error_toast = props.on_error_toast.clone();
        Callback::from(move |_| {
            let Some(api) = api.clone() else {
                on_error_toast.emit("Media API context is unavailable".to_string());
                return;
            };
            let yaml_payload = (*yaml_input).clone();
            let validation_status = validation_status.clone();
            let on_error_toast = on_error_toast.clone();
            spawn_local(async move {
                match validate_yaml(&api.client, yaml_payload).await {
                    Ok(result) => {
                        validation_status.set(Some(format!(
                            "valid={} version={} profiles={} issues={}",
                            result.valid,
                            result.version,
                            result.profile_count,
                            result.issues.join(",")
                        )));
                    }
                    Err(error) => on_error_toast.emit(error),
                }
            });
        })
    };

    let on_apply = {
        let api = api.clone();
        let yaml_input = yaml_input.clone();
        let on_success_toast = props.on_success_toast.clone();
        let on_error_toast = props.on_error_toast.clone();
        let on_refresh = on_refresh.clone();
        Callback::from(move |_| {
            let Some(api) = api.clone() else {
                on_error_toast.emit("Media API context is unavailable".to_string());
                return;
            };
            let yaml_payload = (*yaml_input).clone();
            let on_success_toast = on_success_toast.clone();
            let on_error_toast = on_error_toast.clone();
            let on_refresh = on_refresh.clone();
            spawn_local(async move {
                match apply_yaml(&api.client, yaml_payload).await {
                    Ok(result) => {
                        on_success_toast.emit(format!(
                            "Media YAML applied ({} profiles)",
                            result.media_profile_public_ids.len()
                        ));
                        on_refresh.emit(());
                    }
                    Err(error) => on_error_toast.emit(error),
                }
            });
        })
    };

    let readiness = state
        .readiness
        .as_ref()
        .map(|value| {
            if value.ready {
                "ready".to_string()
            } else {
                value
                    .reason
                    .clone()
                    .unwrap_or_else(|| "not-ready".to_string())
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    html! {
        <section class="space-y-4 p-4" data-testid="media-page">
            <div class="flex items-center gap-2">
                <h1 class="text-2xl font-semibold">{"Media"}</h1>
                <button class="btn btn-sm" onclick={on_refresh_click} disabled={*busy}>{"Refresh"}</button>
                <button class="btn btn-sm" onclick={on_refresh_capability}>{"Refresh capability"}</button>
                <button class="btn btn-sm" onclick={on_export}>{"Export YAML"}</button>
            </div>

            <div class="grid gap-3 md:grid-cols-4">
                <div class="card bg-base-100 shadow"><div class="card-body"><div class="text-xs uppercase opacity-60">{"Profiles"}</div><div class="text-xl">{state.profiles.len()}</div></div></div>
                <div class="card bg-base-100 shadow"><div class="card-body"><div class="text-xs uppercase opacity-60">{"Jobs"}</div><div class="text-xl">{state.jobs.len()}</div></div></div>
                <div class="card bg-base-100 shadow"><div class="card-body"><div class="text-xs uppercase opacity-60">{"Readiness"}</div><div class="text-xl">{readiness}</div></div></div>
                <div class="card bg-base-100 shadow"><div class="card-body"><div class="text-xs uppercase opacity-60">{"Latest codec"}</div><div class="text-xl">{state.latest_capability.as_ref().map(|row| row.codec_name.clone()).unwrap_or_else(|| "-".to_string())}</div></div></div>
            </div>

            <div class="grid gap-3 lg:grid-cols-2">
                <div class="card bg-base-100 shadow">
                    <div class="card-body gap-2">
                        <h2 class="text-lg font-semibold">{"Profiles"}</h2>
                        <ul class="text-sm space-y-1">
                            {for state.profiles.iter().map(|row| html! { <li>{format!("{} ({})", row.profile_key, if row.dry_run_only {"dry-run"} else {"replace"})}</li> })}
                        </ul>
                    </div>
                </div>
                <div class="card bg-base-100 shadow">
                    <div class="card-body gap-2">
                        <h2 class="text-lg font-semibold">{"Recent jobs"}</h2>
                        <ul class="text-sm space-y-1">
                            {for state.jobs.iter().take(10).map(|row| html! { <li>{format!("{} - {}", row.status, row.source_path)}</li> })}
                        </ul>
                    </div>
                </div>
            </div>

            <div class="card bg-base-100 shadow">
                <div class="card-body gap-2">
                    <h2 class="text-lg font-semibold">{"YAML import/export"}</h2>
                    <textarea class="textarea textarea-bordered min-h-48" value={(*yaml_input).clone()} oninput={on_yaml_input} placeholder="Paste Revaer media YAML for validate/apply" />
                    <div class="flex gap-2">
                        <button class="btn btn-sm" onclick={on_validate}>{"Validate YAML"}</button>
                        <button class="btn btn-sm btn-warning" onclick={on_apply}>{"Apply YAML"}</button>
                    </div>
                    {validation_status.as_ref().map(|status| html! { <p class="text-sm">{status.clone()}</p> }).unwrap_or_default()}
                    {state.yaml_export.as_ref().map(|yaml| html! {
                        <details>
                            <summary class="cursor-pointer text-sm">{"Exported YAML"}</summary>
                            <pre class="text-xs overflow-auto">{yaml.clone()}</pre>
                        </details>
                    }).unwrap_or_default()}
                </div>
            </div>
        </section>
    }
}
