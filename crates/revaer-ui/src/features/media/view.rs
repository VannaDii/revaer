use crate::app::api::ApiCtx;
use crate::features::media::api::{
    apply_yaml, create_profile, export_yaml, fetch_compliance, fetch_jobs_for_profiles,
    fetch_latest_capability, fetch_profiles, fetch_readiness, patch_profile, refresh_capability,
    validate_yaml,
};
use crate::features::media::state::MediaViewState;
use crate::models::{MediaProfilePatchRequest, MediaProfileUpsertRequest};
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
    let profile_key = use_state(String::new);
    let source_root = use_state(String::new);
    let output_root = use_state(String::new);
    let retention_days = use_state(|| "30".to_string());
    let dry_run_only = use_state(|| true);
    let compatibility_target_key = use_state(String::new);
    let policy_key = use_state(|| "safe_dry_run".to_string());
    let watcher_enabled = use_state(|| false);
    let schedule_enabled = use_state(|| false);
    let schedule_interval_minutes = use_state(String::new);

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
                let jobs = match profiles.as_ref() {
                    Ok(profiles) => fetch_jobs_for_profiles(&api.client, &profiles.profiles).await,
                    Err(err) => Err(err.clone()),
                };
                let readiness = fetch_readiness(&api.client).await;
                let latest = fetch_latest_capability(&api.client).await;
                let compliance = fetch_compliance(&api.client).await;
                match (profiles, jobs, readiness, latest, compliance) {
                    (Ok(profiles), Ok(jobs), Ok(readiness), Ok(latest), Ok(compliance)) => {
                        let current = (*state).clone();
                        state.set(MediaViewState {
                            profiles: profiles.profiles,
                            jobs: jobs.jobs,
                            readiness: Some(readiness),
                            latest_capability: latest.snapshot,
                            compliance: Some(compliance),
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
    let on_profile_key_input = {
        let profile_key = profile_key.clone();
        Callback::from(move |event: InputEvent| {
            profile_key.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_source_root_input = {
        let source_root = source_root.clone();
        Callback::from(move |event: InputEvent| {
            source_root.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_output_root_input = {
        let output_root = output_root.clone();
        Callback::from(move |event: InputEvent| {
            output_root.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_retention_days_input = {
        let retention_days = retention_days.clone();
        Callback::from(move |event: InputEvent| {
            retention_days.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_dry_run_change = {
        let dry_run_only = dry_run_only.clone();
        Callback::from(move |event: Event| {
            dry_run_only.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .checked(),
            );
        })
    };
    let on_compatibility_target_input = {
        let compatibility_target_key = compatibility_target_key.clone();
        Callback::from(move |event: InputEvent| {
            compatibility_target_key.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_policy_key_input = {
        let policy_key = policy_key.clone();
        Callback::from(move |event: InputEvent| {
            policy_key.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_watcher_change = {
        let watcher_enabled = watcher_enabled.clone();
        Callback::from(move |event: Event| {
            watcher_enabled.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .checked(),
            );
        })
    };
    let on_schedule_change = {
        let schedule_enabled = schedule_enabled.clone();
        Callback::from(move |event: Event| {
            schedule_enabled.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .checked(),
            );
        })
    };
    let on_schedule_interval_input = {
        let schedule_interval_minutes = schedule_interval_minutes.clone();
        Callback::from(move |event: InputEvent| {
            schedule_interval_minutes.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };

    let on_create_profile = {
        let api = api.clone();
        let profile_key = profile_key.clone();
        let source_root = source_root.clone();
        let output_root = output_root.clone();
        let retention_days = retention_days.clone();
        let dry_run_only = dry_run_only.clone();
        let compatibility_target_key = compatibility_target_key.clone();
        let policy_key = policy_key.clone();
        let watcher_enabled = watcher_enabled.clone();
        let schedule_enabled = schedule_enabled.clone();
        let schedule_interval_minutes = schedule_interval_minutes.clone();
        let on_success_toast = props.on_success_toast.clone();
        let on_error_toast = props.on_error_toast.clone();
        let on_refresh = on_refresh.clone();
        Callback::from(move |_| {
            let Some(api) = api.clone() else {
                on_error_toast.emit("Media API context is unavailable".to_string());
                return;
            };
            let retention = (*retention_days).parse::<i32>();
            let Ok(retention_days) = retention else {
                on_error_toast.emit("Retention days must be a whole number".to_string());
                return;
            };
            let schedule_interval = if schedule_interval_minutes.trim().is_empty() {
                None
            } else {
                let interval = (*schedule_interval_minutes).parse::<i32>();
                let Ok(interval) = interval else {
                    on_error_toast.emit("Schedule interval must be a whole number".to_string());
                    return;
                };
                Some(interval)
            };
            let request = MediaProfileUpsertRequest {
                profile_key: (*profile_key).clone(),
                source_root: (*source_root).clone(),
                output_root: (*output_root).clone(),
                dry_run_only: *dry_run_only,
                retention_days,
                compatibility_target_key: empty_string_to_none(
                    (*compatibility_target_key).as_str(),
                ),
                policy_key: (*policy_key).clone(),
                watcher_enabled: *watcher_enabled,
                schedule_enabled: *schedule_enabled,
                schedule_interval_minutes: schedule_interval,
            };
            let on_success_toast = on_success_toast.clone();
            let on_error_toast = on_error_toast.clone();
            let on_refresh = on_refresh.clone();
            spawn_local(async move {
                match create_profile(&api.client, &request).await {
                    Ok(profile) => {
                        on_success_toast.emit(format!("Created profile {}", profile.profile_key));
                        on_refresh.emit(());
                    }
                    Err(error) => on_error_toast.emit(error),
                }
            });
        })
    };

    let on_toggle_profile_dry_run = {
        let api = api.clone();
        let on_success_toast = props.on_success_toast.clone();
        let on_error_toast = props.on_error_toast.clone();
        let on_refresh = on_refresh.clone();
        Callback::from(
            move |(media_profile_public_id, dry_run_only): (uuid::Uuid, bool)| {
                let Some(api) = api.clone() else {
                    on_error_toast.emit("Media API context is unavailable".to_string());
                    return;
                };
                let request = MediaProfilePatchRequest {
                    source_root: None,
                    output_root: None,
                    dry_run_only: Some(dry_run_only),
                    retention_days: None,
                    compatibility_target_key: None,
                    policy_key: None,
                    watcher_enabled: None,
                    schedule_enabled: None,
                    schedule_interval_minutes: None,
                };
                let on_success_toast = on_success_toast.clone();
                let on_error_toast = on_error_toast.clone();
                let on_refresh = on_refresh.clone();
                spawn_local(async move {
                    match patch_profile(&api.client, media_profile_public_id, &request).await {
                        Ok(profile) => {
                            let mode = if profile.dry_run_only {
                                "dry-run"
                            } else {
                                "replace-enabled"
                            };
                            on_success_toast
                                .emit(format!("Profile {} set to {}", profile.profile_key, mode));
                            on_refresh.emit(());
                        }
                        Err(error) => on_error_toast.emit(error),
                    }
                });
            },
        )
    };

    let on_toggle_profile_watcher = {
        let api = api.clone();
        let on_success_toast = props.on_success_toast.clone();
        let on_error_toast = props.on_error_toast.clone();
        let on_refresh = on_refresh.clone();
        Callback::from(
            move |(media_profile_public_id, watcher_enabled): (uuid::Uuid, bool)| {
                let Some(api) = api.clone() else {
                    on_error_toast.emit("Media API context is unavailable".to_string());
                    return;
                };
                let request = MediaProfilePatchRequest {
                    source_root: None,
                    output_root: None,
                    dry_run_only: None,
                    retention_days: None,
                    compatibility_target_key: None,
                    policy_key: None,
                    watcher_enabled: Some(watcher_enabled),
                    schedule_enabled: None,
                    schedule_interval_minutes: None,
                };
                let on_success_toast = on_success_toast.clone();
                let on_error_toast = on_error_toast.clone();
                let on_refresh = on_refresh.clone();
                spawn_local(async move {
                    match patch_profile(&api.client, media_profile_public_id, &request).await {
                        Ok(profile) => {
                            let mode = if profile.watcher_enabled { "on" } else { "off" };
                            on_success_toast
                                .emit(format!("Profile {} watcher {}", profile.profile_key, mode));
                            on_refresh.emit(());
                        }
                        Err(error) => on_error_toast.emit(error),
                    }
                });
            },
        )
    };

    let on_toggle_profile_schedule = {
        let api = api.clone();
        let on_success_toast = props.on_success_toast.clone();
        let on_error_toast = props.on_error_toast.clone();
        let on_refresh = on_refresh.clone();
        Callback::from(
            move |(media_profile_public_id, schedule_enabled): (uuid::Uuid, bool)| {
                let Some(api) = api.clone() else {
                    on_error_toast.emit("Media API context is unavailable".to_string());
                    return;
                };
                let request = MediaProfilePatchRequest {
                    source_root: None,
                    output_root: None,
                    dry_run_only: None,
                    retention_days: None,
                    compatibility_target_key: None,
                    policy_key: None,
                    watcher_enabled: None,
                    schedule_enabled: Some(schedule_enabled),
                    schedule_interval_minutes: None,
                };
                let on_success_toast = on_success_toast.clone();
                let on_error_toast = on_error_toast.clone();
                let on_refresh = on_refresh.clone();
                spawn_local(async move {
                    match patch_profile(&api.client, media_profile_public_id, &request).await {
                        Ok(profile) => {
                            let mode = if profile.schedule_enabled {
                                "on"
                            } else {
                                "off"
                            };
                            on_success_toast
                                .emit(format!("Profile {} schedule {}", profile.profile_key, mode));
                            on_refresh.emit(());
                        }
                        Err(error) => on_error_toast.emit(error),
                    }
                });
            },
        )
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

            <div class="card bg-base-100 shadow">
                <div class="card-body gap-2">
                    <h2 class="text-lg font-semibold">{"Compliance"}</h2>
                    {state.compliance.as_ref().map(|compliance| html! {
                        <div class="grid gap-2 text-sm md:grid-cols-2" data-testid="media-compliance-panel">
                            <div><span class="font-medium">{"License mode"}</span><span class="ml-2">{compliance.license_mode.clone()}</span></div>
                            <div><span class="font-medium">{"Source offer"}</span><span class="ml-2 break-all">{compliance.source_offer_path.clone()}</span></div>
                            <div><span class="font-medium">{"Third-party notices"}</span><span class="ml-2 break-all">{compliance.third_party_notices_path.clone()}</span></div>
                            <div><span class="font-medium">{"SBOM"}</span><span class="ml-2 break-all">{compliance.sbom_path.clone()}</span></div>
                            <div><span class="font-medium">{"Inventory"}</span><span class="ml-2 break-all">{compliance.inventory_path.clone()}</span></div>
                            <div><span class="font-medium">{"ExifTool exception"}</span><span class="ml-2 break-all">{compliance.exiftool_exception_path.clone()}</span></div>
                            <div class="md:col-span-2"><span class="font-medium">{"Excluded capabilities"}</span><span class="ml-2">{compliance.license_excluded_capabilities.join(", ")}</span></div>
                        </div>
                    }).unwrap_or_else(|| html! {
                        <div class="text-sm" data-testid="media-compliance-panel">{"License mode unknown"}</div>
                    })}
                </div>
            </div>

            <div class="grid gap-3 lg:grid-cols-2">
                <div class="card bg-base-100 shadow">
                    <div class="card-body gap-2">
                        <h2 class="text-lg font-semibold">{"Profiles"}</h2>
                        <div class="grid gap-2 md:grid-cols-2" data-testid="media-profile-form">
                            <input class="input input-bordered input-sm" placeholder="profile_key" value={(*profile_key).clone()} oninput={on_profile_key_input} />
                            <input class="input input-bordered input-sm" placeholder="source_root" value={(*source_root).clone()} oninput={on_source_root_input} />
                            <input class="input input-bordered input-sm" placeholder="output_root" value={(*output_root).clone()} oninput={on_output_root_input} />
                            <input class="input input-bordered input-sm" placeholder="retention_days" value={(*retention_days).clone()} oninput={on_retention_days_input} />
                            <input class="input input-bordered input-sm" placeholder="compatibility_target_key" value={(*compatibility_target_key).clone()} oninput={on_compatibility_target_input} />
                            <input class="input input-bordered input-sm" placeholder="policy_key" value={(*policy_key).clone()} oninput={on_policy_key_input} />
                            <input class="input input-bordered input-sm" placeholder="schedule_interval_minutes" value={(*schedule_interval_minutes).clone()} oninput={on_schedule_interval_input} />
                            <label class="label cursor-pointer gap-2 justify-start">
                                <input type="checkbox" class="checkbox checkbox-sm" checked={*dry_run_only} onchange={on_dry_run_change} />
                                <span class="label-text">{"Dry run only"}</span>
                            </label>
                            <label class="label cursor-pointer gap-2 justify-start">
                                <input type="checkbox" class="checkbox checkbox-sm" checked={*watcher_enabled} onchange={on_watcher_change} />
                                <span class="label-text">{"Enable watcher"}</span>
                            </label>
                            <label class="label cursor-pointer gap-2 justify-start">
                                <input type="checkbox" class="checkbox checkbox-sm" checked={*schedule_enabled} onchange={on_schedule_change} />
                                <span class="label-text">{"Enable schedule"}</span>
                            </label>
                            <button class="btn btn-sm btn-primary" onclick={on_create_profile}>{"Create profile"}</button>
                        </div>
                        <ul class="text-sm space-y-1">
                            {for state.profiles.iter().map(|row| {
                                let on_toggle_profile_dry_run = on_toggle_profile_dry_run.clone();
                                let on_toggle_profile_watcher = on_toggle_profile_watcher.clone();
                                let on_toggle_profile_schedule = on_toggle_profile_schedule.clone();
                                let media_profile_public_id = row.media_profile_public_id;
                                let next_dry_run_only = !row.dry_run_only;
                                let next_watcher_enabled = !row.watcher_enabled;
                                let next_schedule_enabled = !row.schedule_enabled;
                                let schedule_toggle_disabled = !row.schedule_enabled && row.schedule_interval_minutes.is_none();
                                html! {
                                    <li class="flex flex-wrap items-center gap-2">
                                        <span>{format!("{} ({})", row.profile_key, if row.dry_run_only {"dry-run"} else {"replace"})}</span>
                                        <span class="opacity-70">{format!("src={} out={} retention={}d target={} policy={} watcher={} schedule={}",
                                            row.source_root,
                                            row.output_root,
                                            row.retention_days,
                                            row.compatibility_target_key.clone().unwrap_or_else(|| "none".to_string()),
                                            row.policy_key,
                                            if row.watcher_enabled {"on"} else {"off"},
                                            describe_schedule(row.schedule_enabled, row.schedule_interval_minutes))}</span>
                                        <button
                                            class="btn btn-xs"
                                            onclick={Callback::from(move |_| on_toggle_profile_dry_run.emit((media_profile_public_id, next_dry_run_only)))}
                                        >
                                            {if row.dry_run_only {"Enable replace"} else {"Set dry-run"}}
                                        </button>
                                        <button
                                            class="btn btn-xs"
                                            onclick={Callback::from(move |_| on_toggle_profile_watcher.emit((media_profile_public_id, next_watcher_enabled)))}
                                        >
                                            {if row.watcher_enabled {"Disable watcher"} else {"Enable watcher"}}
                                        </button>
                                        <button
                                            class="btn btn-xs"
                                            disabled={schedule_toggle_disabled}
                                            onclick={Callback::from(move |_| on_toggle_profile_schedule.emit((media_profile_public_id, next_schedule_enabled)))}
                                        >
                                            {if row.schedule_enabled {"Disable schedule"} else {"Enable schedule"}}
                                        </button>
                                    </li>
                                }
                            })}
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

fn empty_string_to_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn describe_schedule(enabled: bool, interval: Option<i32>) -> String {
    if !enabled {
        return "off".to_string();
    }
    interval
        .map(|minutes| format!("{minutes}m"))
        .unwrap_or_else(|| "enabled".to_string())
}
