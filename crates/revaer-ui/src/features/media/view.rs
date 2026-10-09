use crate::app::api::ApiCtx;
use crate::features::media::api::{
    apply_yaml, export_yaml, fetch_compatibility_targets, fetch_compliance, fetch_job_diagnostics,
    fetch_latest_capability, fetch_policies, fetch_readiness, fetch_recent_jobs,
    refresh_capability, upsert_compatibility_target, upsert_policy, validate_yaml,
};
use crate::features::media::logic::summarize_media_job_diagnostics;
use crate::features::media::state::{
    MediaJobDiagnosticsMap, MediaJobDiagnosticsState, MediaViewState,
    is_current_diagnostics_request,
};
use crate::models::{
    MediaCompatibilityTargetResponse, MediaCompatibilityTargetUpsertRequest,
    MediaJobArtifactResponse, MediaJobCompactAuditResponse, MediaJobOperationResponse,
    MediaJobPlanReasonResponse, MediaJobVerificationCheckResponse, MediaJobViolationResponse,
    MediaPolicyResponse, MediaPolicyUpsertRequest,
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use yew::platform::spawn_local;
use yew::prelude::*;

const RECENT_MEDIA_JOBS_LIMIT: u16 = 10;
type MediaJobDiagnosticsRef = Rc<RefCell<MediaJobDiagnosticsMap>>;
type ActiveDiagnosticsRequests = Rc<RefCell<HashMap<uuid::Uuid, uuid::Uuid>>>;
type OpenedJobDiagnosticsRef = Rc<RefCell<HashSet<uuid::Uuid>>>;

#[derive(Clone)]
struct JobDiagnosticsHandles {
    job_diagnostics: UseStateHandle<MediaJobDiagnosticsMap>,
    job_diagnostics_ref: MediaJobDiagnosticsRef,
    opened_job_diagnostics: UseStateHandle<HashSet<uuid::Uuid>>,
    opened_job_diagnostics_ref: OpenedJobDiagnosticsRef,
    active_diagnostics_requests: ActiveDiagnosticsRequests,
}

#[derive(Properties, PartialEq)]
pub(crate) struct MediaPageProps {
    pub on_success_toast: Callback<String>,
    pub on_error_toast: Callback<String>,
}

#[derive(Clone)]
struct ToastCallbacks {
    success: Callback<String>,
    error: Callback<String>,
}

#[derive(Clone)]
struct TargetCatalogFormHandles {
    key: UseStateHandle<String>,
    version: UseStateHandle<String>,
    display_name: UseStateHandle<String>,
    video_codec: UseStateHandle<String>,
    audio_codec: UseStateHandle<String>,
    audio_channels: UseStateHandle<String>,
    audio_channel_layout: UseStateHandle<String>,
    subtitle_policy: UseStateHandle<String>,
}

#[derive(Clone)]
struct PolicyCatalogFormHandles {
    output: UseStateHandle<super::policy_output_view::PolicyOutputDraft>,
    key: UseStateHandle<String>,
    version: UseStateHandle<String>,
    display_name: UseStateHandle<String>,
    video_intent: UseStateHandle<String>,
    verification_strictness: UseStateHandle<String>,
    verification_duration_tolerance_millis: UseStateHandle<String>,
    verification_mux_validation: UseStateHandle<bool>,
    verification_decode_all_streams: UseStateHandle<bool>,
    verification_keyframe_seek: UseStateHandle<bool>,
    verification_playback_probe: UseStateHandle<bool>,
}

#[function_component(MediaPage)]
pub(crate) fn media_page(props: &MediaPageProps) -> Html {
    let api = use_context::<ApiCtx>();
    let state = use_state(MediaViewState::default);
    let job_diagnostics = use_state(MediaJobDiagnosticsMap::new);
    let job_diagnostics_ref = use_mut_ref(MediaJobDiagnosticsMap::new);
    let opened_job_diagnostics = use_state(HashSet::<uuid::Uuid>::new);
    let opened_job_diagnostics_ref = use_mut_ref(HashSet::<uuid::Uuid>::new);
    let active_diagnostics_requests = use_mut_ref(HashMap::<uuid::Uuid, uuid::Uuid>::new);
    let busy = use_state(|| false);
    let yaml_input = use_state(String::new);
    let validation_status = use_state(|| None::<String>);
    let target_catalog_key = use_state(String::new);
    let target_catalog_version = use_state(|| "1".to_string());
    let target_catalog_display_name = use_state(String::new);
    let target_catalog_video_codec = use_state(|| "hevc".to_string());
    let target_catalog_audio_codec = use_state(|| "aac".to_string());
    let target_catalog_audio_channels = use_state(String::new);
    let target_catalog_audio_channel_layout = use_state(String::new);
    let target_catalog_subtitle_policy = use_state(|| "selected".to_string());
    let policy_catalog_key = use_state(String::new);
    let policy_output = use_state(super::policy_output_view::PolicyOutputDraft::default);
    let policy_catalog_version = use_state(|| "1".to_string());
    let policy_catalog_display_name = use_state(String::new);
    let policy_catalog_video_intent = use_state(|| "general".to_string());
    let policy_verification_strictness = use_state(|| "strict".to_string());
    let policy_verification_duration_tolerance_millis = use_state(|| "100".to_string());
    let policy_verification_mux_validation = use_state(|| true);
    let policy_verification_decode_all_streams = use_state(|| true);
    let policy_verification_keyframe_seek = use_state(|| true);
    let policy_verification_playback_probe = use_state(|| true);
    let toasts = ToastCallbacks {
        success: props.on_success_toast.clone(),
        error: props.on_error_toast.clone(),
    };
    let target_form = TargetCatalogFormHandles {
        key: target_catalog_key.clone(),
        version: target_catalog_version.clone(),
        display_name: target_catalog_display_name.clone(),
        video_codec: target_catalog_video_codec.clone(),
        audio_codec: target_catalog_audio_codec.clone(),
        audio_channels: target_catalog_audio_channels.clone(),
        audio_channel_layout: target_catalog_audio_channel_layout.clone(),
        subtitle_policy: target_catalog_subtitle_policy.clone(),
    };
    let policy_form = PolicyCatalogFormHandles {
        output: policy_output.clone(),
        key: policy_catalog_key.clone(),
        version: policy_catalog_version.clone(),
        display_name: policy_catalog_display_name.clone(),
        video_intent: policy_catalog_video_intent.clone(),
        verification_strictness: policy_verification_strictness.clone(),
        verification_duration_tolerance_millis: policy_verification_duration_tolerance_millis
            .clone(),
        verification_mux_validation: policy_verification_mux_validation.clone(),
        verification_decode_all_streams: policy_verification_decode_all_streams.clone(),
        verification_keyframe_seek: policy_verification_keyframe_seek.clone(),
        verification_playback_probe: policy_verification_playback_probe.clone(),
    };
    let diagnostics = JobDiagnosticsHandles {
        job_diagnostics: job_diagnostics.clone(),
        job_diagnostics_ref,
        opened_job_diagnostics: opened_job_diagnostics.clone(),
        opened_job_diagnostics_ref,
        active_diagnostics_requests,
    };
    let on_refresh = build_refresh_callback(
        api.clone(),
        state.clone(),
        diagnostics.clone(),
        busy.clone(),
        toasts.error.clone(),
    );
    let on_toggle_job_diagnostics =
        build_job_diagnostics_toggle_callback(api.clone(), diagnostics, toasts.error.clone());

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

    let on_refresh_capability =
        build_refresh_capability_callback(api.clone(), toasts.clone(), on_refresh.clone());
    let on_export = build_export_callback(api.clone(), state.clone(), toasts.clone());

    let on_yaml_input = {
        let yaml_input = yaml_input.clone();
        Callback::from(move |event: InputEvent| {
            let value = event
                .target_unchecked_into::<web_sys::HtmlTextAreaElement>()
                .value();
            yaml_input.set(value);
        })
    };

    let on_validate = build_validate_callback(
        api.clone(),
        yaml_input.clone(),
        validation_status.clone(),
        toasts.error.clone(),
    );
    let on_apply = build_apply_callback(
        api.clone(),
        yaml_input.clone(),
        toasts.clone(),
        on_refresh.clone(),
    );
    let on_target_catalog_key_input = {
        let target_catalog_key = target_catalog_key.clone();
        Callback::from(move |event: InputEvent| {
            target_catalog_key.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_target_catalog_version_input = {
        let target_catalog_version = target_catalog_version.clone();
        Callback::from(move |event: InputEvent| {
            target_catalog_version.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_target_catalog_display_name_input = {
        let target_catalog_display_name = target_catalog_display_name.clone();
        Callback::from(move |event: InputEvent| {
            target_catalog_display_name.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_target_catalog_video_codec_input = {
        let target_catalog_video_codec = target_catalog_video_codec.clone();
        Callback::from(move |event: InputEvent| {
            target_catalog_video_codec.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_target_catalog_audio_codec_input = {
        let target_catalog_audio_codec = target_catalog_audio_codec.clone();
        Callback::from(move |event: InputEvent| {
            target_catalog_audio_codec.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_target_catalog_audio_channels_input = {
        let target_catalog_audio_channels = target_catalog_audio_channels.clone();
        Callback::from(move |event: InputEvent| {
            target_catalog_audio_channels.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_target_catalog_audio_channel_layout_input = {
        let target_catalog_audio_channel_layout = target_catalog_audio_channel_layout.clone();
        Callback::from(move |event: InputEvent| {
            target_catalog_audio_channel_layout.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_target_catalog_subtitle_policy_change = {
        let target_catalog_subtitle_policy = target_catalog_subtitle_policy.clone();
        Callback::from(move |event: Event| {
            target_catalog_subtitle_policy.set(
                event
                    .target_unchecked_into::<web_sys::HtmlSelectElement>()
                    .value(),
            );
        })
    };
    let on_policy_catalog_key_input = {
        let policy_catalog_key = policy_catalog_key.clone();
        Callback::from(move |event: InputEvent| {
            policy_catalog_key.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_policy_catalog_version_input = {
        let policy_catalog_version = policy_catalog_version.clone();
        Callback::from(move |event: InputEvent| {
            policy_catalog_version.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_policy_catalog_display_name_input = {
        let policy_catalog_display_name = policy_catalog_display_name.clone();
        Callback::from(move |event: InputEvent| {
            policy_catalog_display_name.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_policy_catalog_video_intent_change = {
        let policy_catalog_video_intent = policy_catalog_video_intent.clone();
        Callback::from(move |event: Event| {
            policy_catalog_video_intent.set(
                event
                    .target_unchecked_into::<web_sys::HtmlSelectElement>()
                    .value(),
            );
        })
    };
    let on_policy_verification_strictness_change = {
        let value = policy_verification_strictness.clone();
        Callback::from(move |event: Event| {
            value.set(
                event
                    .target_unchecked_into::<web_sys::HtmlSelectElement>()
                    .value(),
            );
        })
    };
    let on_policy_verification_duration_input = {
        let value = policy_verification_duration_tolerance_millis.clone();
        Callback::from(move |event: InputEvent| {
            value.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    let on_policy_mux_validation_change =
        bool_input_callback(policy_verification_mux_validation.clone());
    let on_policy_decode_all_streams_change =
        bool_input_callback(policy_verification_decode_all_streams.clone());
    let on_policy_keyframe_seek_change =
        bool_input_callback(policy_verification_keyframe_seek.clone());
    let on_policy_playback_probe_change =
        bool_input_callback(policy_verification_playback_probe.clone());
    let on_save_target = build_save_target_callback(
        api.clone(),
        state.clone(),
        busy.clone(),
        target_form.clone(),
        toasts.clone(),
    );
    let on_save_policy = build_save_policy_callback(
        api.clone(),
        state.clone(),
        busy.clone(),
        policy_form.clone(),
        toasts.clone(),
    );
    let readiness = display_readiness(&state);
    let capability_codecs = display_capability_codecs(&state);

    html! {
        <section class="space-y-4 p-4" data-testid="media-page">
            <div class="flex items-center gap-2">
                <h1 class="text-2xl font-semibold">{"Media"}</h1>
                <button class="btn btn-sm" onclick={on_refresh_click} disabled={*busy}>{"Refresh"}</button>
                <button class="btn btn-sm" onclick={on_refresh_capability}>{"Refresh capability"}</button>
                <button class="btn btn-sm" onclick={on_export}>{"Export YAML"}</button>
            </div>

            <super::root_readiness_view::RootReadiness />
            <super::root_catalog_view::RootConfiguration />

            <div class="grid gap-3 md:grid-cols-3">
                <div class="card bg-base-100 shadow"><div class="card-body"><div class="text-xs uppercase opacity-60">{"Jobs"}</div><div class="text-xl">{state.jobs.len()}</div></div></div>
                <div class="card bg-base-100 shadow"><div class="card-body"><div class="text-xs uppercase opacity-60">{"Readiness"}</div><div class="text-xl">{readiness}</div></div></div>
                <div class="card bg-base-100 shadow"><div class="card-body"><div class="text-xs uppercase opacity-60">{"Capability codecs"}</div><div class="text-xl">{capability_codecs}</div></div></div>
            </div>

            <div class="card bg-base-100 shadow">
                <div class="card-body gap-2">
                    <h2 class="text-lg font-semibold">{"Compliance"}</h2>
                    {state.compliance.as_ref().map(|compliance| html! {
                        <div class="grid gap-2 text-sm md:grid-cols-2" data-testid="media-compliance-panel">
                            <div><span class="font-medium">{"License mode"}</span><span class="ml-2">{compliance.license_mode.clone()}</span></div>
                            <div><span class="font-medium">{"Image mode"}</span><span class="ml-2">{compliance.image_license_mode.clone()}</span></div>
                            <div><span class="font-medium">{"FFmpeg mode"}</span><span class="ml-2">{compliance.ffmpeg_license_mode.clone()}</span></div>
                            <div><span class="font-medium">{"FFmpeg flags"}</span><span class="ml-2">{format!("gpl={} version3={} nonfree={}", compliance.ffmpeg_enable_gpl, compliance.ffmpeg_enable_version3, compliance.ffmpeg_enable_nonfree)}</span></div>
                            <div><span class="font-medium">{"Source offer"}</span><span class="ml-2 break-all">{compliance.source_offer_path.clone()}</span></div>
                            <div><span class="font-medium">{"Third-party notices"}</span><span class="ml-2 break-all">{compliance.third_party_notices_path.clone()}</span></div>
                            <div><span class="font-medium">{"SBOM"}</span><span class="ml-2 break-all">{compliance.sbom_path.clone()}</span></div>
                            <div><span class="font-medium">{"Inventory"}</span><span class="ml-2 break-all">{compliance.inventory_path.clone()}</span></div>
                            <div><span class="font-medium">{"ExifTool exception"}</span><span class="ml-2 break-all">{compliance.exiftool_exception_path.clone()}</span></div>
                            <div><span class="font-medium">{"Compliance bundle"}</span><span class="ml-2 break-all">{format!("{} {}", compliance.source_compliance_bundle_digest, compliance.source_compliance_bundle_path)}</span></div>
                            <div class="md:col-span-2"><span class="font-medium">{"Excluded capabilities"}</span><span class="ml-2">{compliance.license_excluded_capabilities.join(", ")}</span></div>
                            <div class="md:col-span-2"><span class="font-medium">{"Absent excluded capabilities"}</span><span class="ml-2">{compliance.absent_license_excluded_capabilities.join(", ")}</span></div>
                        </div>
                    }).unwrap_or_else(|| html! {
                        <div class="text-sm" data-testid="media-compliance-panel">{"License mode unknown"}</div>
                    })}
                </div>
            </div>

            <div class="grid gap-3 lg:grid-cols-2">
                <div class="card bg-base-100 shadow">
                    <div class="card-body gap-2">
                        <h2 class="text-lg font-semibold">{"Compatibility targets"}</h2>
                        <ul class="text-sm space-y-1" data-testid="media-target-catalog">
                            {for state.compatibility_targets.iter().map(|target| html! {
                                <li class="flex flex-wrap gap-2">
                                    <span class="font-medium">{target.compatibility_target_key.clone()}</span>
                                    <span class="opacity-70">{format!("v{} {} {}/{} channels={} layout={} subtitles={}", target.version, target.display_name, target.video_codec, target.audio_codec, display_optional_i32(target.audio_channels), display_optional_string(target.audio_channel_layout.as_deref()), target.subtitle_policy)}</span>
                                </li>
                            })}
                        </ul>
                        <div class="grid gap-2 md:grid-cols-2" data-testid="media-target-form">
                            <input class="input input-bordered input-sm" placeholder="target_key" value={(*target_catalog_key).clone()} oninput={on_target_catalog_key_input} />
                            <input class="input input-bordered input-sm" placeholder="target_version" value={(*target_catalog_version).clone()} oninput={on_target_catalog_version_input} />
                            <input class="input input-bordered input-sm" placeholder="target_display_name" value={(*target_catalog_display_name).clone()} oninput={on_target_catalog_display_name_input} />
                            <input class="input input-bordered input-sm" placeholder="target_video_codec" value={(*target_catalog_video_codec).clone()} oninput={on_target_catalog_video_codec_input} />
                            <input class="input input-bordered input-sm" placeholder="target_audio_codec" value={(*target_catalog_audio_codec).clone()} oninput={on_target_catalog_audio_codec_input} />
                            <input class="input input-bordered input-sm" placeholder="target_audio_channels" value={(*target_catalog_audio_channels).clone()} oninput={on_target_catalog_audio_channels_input} />
                            <input class="input input-bordered input-sm" placeholder="target_audio_channel_layout" value={(*target_catalog_audio_channel_layout).clone()} oninput={on_target_catalog_audio_channel_layout_input} />
                            <select class="select select-bordered select-sm" aria-label="target_subtitle_policy" value={(*target_catalog_subtitle_policy).clone()} onchange={on_target_catalog_subtitle_policy_change}>
                                <option value="selected">{"Selected subtitles"}</option>
                                <option value="all">{"All subtitles"}</option>
                                <option value="none">{"No subtitles"}</option>
                            </select>
                            <button class="btn btn-sm btn-primary md:col-span-2" onclick={on_save_target}>{"Save target"}</button>
                        </div>
                    </div>
                </div>
                <div class="card bg-base-100 shadow">
                    <div class="card-body gap-2">
                        <h2 class="text-lg font-semibold">{"Policies"}</h2>
                        <ul class="text-sm space-y-1" data-testid="media-policy-catalog">
                            {for state.policies.iter().map(|policy| html! {
                                <li class="flex flex-wrap gap-2">
                                    <span class="font-medium">{policy.policy_key.clone()}</span>
                                    <span>{format!("{} | Replacement: {} | Quarantine: {} | Permissions: {} | Ownership: {}",
                                        if policy.output.dry_run { "Dry-run" } else { "Execution" },
                                        policy.output.replacement_mode,
                                        policy.output.quarantine_enabled,
                                        policy.output.preservation.preserve_permissions,
                                        policy.output.preservation.preserve_ownership)}</span>
                                    <span class="opacity-70">{format!("v{} {} intent={} verification={} tolerance={}ms mux={} decode={} seek={} playback={}", policy.version, policy.display_name, policy.video_intent, policy.verification_strictness, policy.verification_duration_tolerance_millis, policy.verification_mux_validation, policy.verification_decode_all_streams, policy.verification_keyframe_seek, policy.verification_playback_probe)}</span>
                                </li>
                            })}
                        </ul>
                        <div class="grid gap-2 md:grid-cols-2" data-testid="media-policy-form">
                            {super::policy_output_view::fields(&policy_output)}
                            <input class="input input-bordered input-sm" placeholder="policy_catalog_key" value={(*policy_catalog_key).clone()} oninput={on_policy_catalog_key_input} />
                            <input class="input input-bordered input-sm" placeholder="policy_version" value={(*policy_catalog_version).clone()} oninput={on_policy_catalog_version_input} />
                            <input class="input input-bordered input-sm" placeholder="policy_display_name" value={(*policy_catalog_display_name).clone()} oninput={on_policy_catalog_display_name_input} />
                            <select class="select select-bordered select-sm" aria-label="policy_video_intent" value={(*policy_catalog_video_intent).clone()} onchange={on_policy_catalog_video_intent_change}>
                                <option value="general">{"General"}</option>
                                <option value="anime">{"Anime"}</option>
                                <option value="archival">{"Archival"}</option>
                            </select>
                            <select class="select select-bordered select-sm" aria-label="policy_verification_strictness" value={(*policy_verification_strictness).clone()} onchange={on_policy_verification_strictness_change}>
                                <option value="strict">{"Strict"}</option>
                                <option value="balanced">{"Balanced"}</option>
                                <option value="fast">{"Fast"}</option>
                            </select>
                            <input class="input input-bordered input-sm" aria-label="policy_verification_duration_tolerance_millis" placeholder="duration_tolerance_millis" value={(*policy_verification_duration_tolerance_millis).clone()} oninput={on_policy_verification_duration_input} />
                            <label class="label cursor-pointer gap-2 justify-start">
                                <input type="checkbox" class="checkbox checkbox-sm" checked={*policy_verification_mux_validation} onchange={on_policy_mux_validation_change} />
                                <span class="label-text">{"Validate mux structure"}</span>
                            </label>
                            <label class="label cursor-pointer gap-2 justify-start">
                                <input type="checkbox" class="checkbox checkbox-sm" checked={*policy_verification_decode_all_streams} onchange={on_policy_decode_all_streams_change} />
                                <span class="label-text">{"Decode all streams"}</span>
                            </label>
                            <label class="label cursor-pointer gap-2 justify-start">
                                <input type="checkbox" class="checkbox checkbox-sm" checked={*policy_verification_keyframe_seek} onchange={on_policy_keyframe_seek_change} />
                                <span class="label-text">{"Verify midpoint seek"}</span>
                            </label>
                            <label class="label cursor-pointer gap-2 justify-start">
                                <input type="checkbox" class="checkbox checkbox-sm" checked={*policy_verification_playback_probe} onchange={on_policy_playback_probe_change} />
                                <span class="label-text">{"Run playback probe"}</span>
                            </label>
                            <button class="btn btn-sm btn-primary md:col-span-2" onclick={on_save_policy}>{"Save policy"}</button>
                        </div>
                    </div>
                </div>
            </div>

            <div class="grid gap-3 lg:grid-cols-2">
                <div class="card bg-base-100 shadow">
                    <div class="card-body gap-2">
                        <h2 class="text-lg font-semibold">{"Recent jobs"}</h2>
                        <ul class="text-sm space-y-1">
                            {for state.jobs.iter().take(10).map(|row| {
                                let media_job_public_id = row.media_job_public_id;
                                let is_open = opened_job_diagnostics.contains(&media_job_public_id);
                                let diagnostics = job_diagnostics.get(&media_job_public_id);
                                let on_toggle_job_diagnostics = on_toggle_job_diagnostics.clone();
                                let on_toggle = Callback::from(move |event: MouseEvent| {
                                    event.prevent_default();
                                    on_toggle_job_diagnostics.emit(media_job_public_id);
                                });
                                html! {
                                    <li key={media_job_public_id.to_string()} data-job-id={media_job_public_id.to_string()}>
                                        <details class="collapse collapse-arrow bg-base-200" open={is_open} data-testid="media-job-diagnostics">
                                            <summary class="collapse-title text-sm font-medium" onclick={on_toggle}>
                                                {format!("{} - {}", row.status, row.source_path)}
                                            </summary>
                                            <super::job_action_view::MediaJobAction job_id={media_job_public_id}
                                                status={row.status.clone()} on_changed={on_refresh.clone()} />
                                            {render_job_diagnostics_content(diagnostics)}
                                        </details>
                                    </li>
                                }
                            })}
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

fn parse_catalog_version(value: &str, label: &str) -> Result<i32, String> {
    let version = value
        .trim()
        .parse::<i32>()
        .map_err(|_| format!("{label} must be a positive integer"))?;
    if version > 0 {
        Ok(version)
    } else {
        Err(format!("{label} must be a positive integer"))
    }
}

fn parse_bounded_nonnegative_i64(value: &str, label: &str, maximum: i64) -> Result<i64, String> {
    let parsed = value
        .trim()
        .parse::<i64>()
        .map_err(|_| format!("{label} must be an integer between 0 and {maximum}"))?;
    if (0..=maximum).contains(&parsed) {
        Ok(parsed)
    } else {
        Err(format!(
            "{label} must be an integer between 0 and {maximum}"
        ))
    }
}

fn bool_input_callback(value: UseStateHandle<bool>) -> Callback<Event> {
    Callback::from(move |event: Event| {
        value.set(
            event
                .target_unchecked_into::<web_sys::HtmlInputElement>()
                .checked(),
        );
    })
}

fn parse_optional_positive_i32(value: &str, label: &str) -> Result<Option<i32>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    parse_catalog_version(trimmed, label).map(Some)
}

fn display_optional_i32(value: Option<i32>) -> String {
    value
        .map(|item| item.to_string())
        .unwrap_or_else(|| "preserve".to_string())
}

fn display_optional_string(value: Option<&str>) -> String {
    value
        .map(str::to_string)
        .unwrap_or_else(|| "preserve".to_string())
}

fn display_readiness(state: &MediaViewState) -> String {
    state
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
        .unwrap_or_else(|| "unknown".to_string())
}

fn display_capability_codecs(state: &MediaViewState) -> String {
    state
        .latest_capability
        .as_ref()
        .map(|row| row.codecs.len().to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn api_context(api: Option<ApiCtx>, on_error_toast: &Callback<String>) -> Option<ApiCtx> {
    if api.is_none() {
        on_error_toast.emit("Media API context is unavailable".to_string());
    }
    api
}

fn emit_parse_error<T, E: ToString>(
    result: Result<T, E>,
    on_error: &Callback<String>,
) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(message) => {
            on_error.emit(message.to_string());
            None
        }
    }
}

fn build_refresh_callback(
    api: Option<ApiCtx>,
    state: UseStateHandle<MediaViewState>,
    diagnostics: JobDiagnosticsHandles,
    busy: UseStateHandle<bool>,
    on_error_toast: Callback<String>,
) -> Callback<()> {
    let JobDiagnosticsHandles {
        job_diagnostics,
        job_diagnostics_ref,
        opened_job_diagnostics,
        opened_job_diagnostics_ref,
        active_diagnostics_requests,
    } = diagnostics;
    Callback::from(move |_| {
        let Some(api) = api_context(api.clone(), &on_error_toast) else {
            return;
        };
        busy.set(true);
        let state = state.clone();
        let job_diagnostics = job_diagnostics.clone();
        let job_diagnostics_ref = job_diagnostics_ref.clone();
        let opened_job_diagnostics = opened_job_diagnostics.clone();
        let opened_job_diagnostics_ref = opened_job_diagnostics_ref.clone();
        let active_diagnostics_requests = active_diagnostics_requests.clone();
        let busy = busy.clone();
        let on_error_toast = on_error_toast.clone();
        spawn_local(async move {
            let jobs = fetch_recent_jobs(&api.client, RECENT_MEDIA_JOBS_LIMIT, None, None).await;
            let readiness = fetch_readiness(&api.client).await;
            let latest = fetch_latest_capability(&api.client).await;
            let compliance = fetch_compliance(&api.client).await;
            let compatibility_targets = fetch_compatibility_targets(&api.client).await;
            let policies = fetch_policies(&api.client).await;
            match (
                jobs,
                readiness,
                latest,
                compliance,
                compatibility_targets,
                policies,
            ) {
                (
                    Ok(jobs),
                    Ok(readiness),
                    Ok(latest),
                    Ok(compliance),
                    Ok(compatibility_targets),
                    Ok(policies),
                ) => {
                    let current = (*state).clone();
                    let current_job_ids = jobs
                        .jobs
                        .iter()
                        .map(|summary| summary.job.media_job_public_id)
                        .collect::<HashSet<_>>();
                    let next_job_diagnostics = {
                        let mut diagnostics = job_diagnostics_ref.borrow_mut();
                        diagnostics.retain(|job_id, _| current_job_ids.contains(job_id));
                        diagnostics.clone()
                    };
                    job_diagnostics.set(next_job_diagnostics);
                    active_diagnostics_requests
                        .borrow_mut()
                        .retain(|job_id, _| current_job_ids.contains(job_id));
                    let next_opened_job_diagnostics = {
                        let mut opened = opened_job_diagnostics_ref.borrow_mut();
                        opened.retain(|job_id| current_job_ids.contains(job_id));
                        opened.clone()
                    };
                    opened_job_diagnostics.set(next_opened_job_diagnostics);
                    state.set(MediaViewState {
                        jobs: jobs.jobs.into_iter().map(|summary| summary.job).collect(),
                        readiness: Some(readiness),
                        latest_capability: latest.snapshot,
                        compliance: Some(compliance),
                        compatibility_targets: compatibility_targets.targets,
                        policies: policies.policies,
                        yaml_export: current.yaml_export,
                    });
                }
                _ => on_error_toast.emit("Failed to refresh media snapshot".to_string()),
            }
            busy.set(false);
        });
    })
}

fn build_refresh_capability_callback(
    api: Option<ApiCtx>,
    toasts: ToastCallbacks,
    on_refresh: Callback<()>,
) -> Callback<MouseEvent> {
    Callback::from(move |_| {
        let Some(api) = api_context(api.clone(), &toasts.error) else {
            return;
        };
        let toasts = toasts.clone();
        let on_refresh = on_refresh.clone();
        spawn_local(async move {
            match refresh_capability(&api.client).await {
                Ok(_) => {
                    toasts
                        .success
                        .emit("Media capability refreshed".to_string());
                    on_refresh.emit(());
                }
                Err(error) => toasts.error.emit(error),
            }
        });
    })
}

fn build_export_callback(
    api: Option<ApiCtx>,
    state: UseStateHandle<MediaViewState>,
    toasts: ToastCallbacks,
) -> Callback<MouseEvent> {
    Callback::from(move |_| {
        let Some(api) = api_context(api.clone(), &toasts.error) else {
            return;
        };
        let state = state.clone();
        let toasts = toasts.clone();
        spawn_local(async move {
            match export_yaml(&api.client).await {
                Ok(response) => {
                    let mut next = (*state).clone();
                    next.yaml_export = Some(response.yaml_payload);
                    state.set(next);
                    toasts.success.emit("Media YAML exported".to_string());
                }
                Err(error) => toasts.error.emit(error),
            }
        });
    })
}

fn build_validate_callback(
    api: Option<ApiCtx>,
    yaml_input: UseStateHandle<String>,
    validation_status: UseStateHandle<Option<String>>,
    on_error_toast: Callback<String>,
) -> Callback<MouseEvent> {
    Callback::from(move |_| {
        let Some(api) = api_context(api.clone(), &on_error_toast) else {
            return;
        };
        let yaml_payload = (*yaml_input).clone();
        let validation_status = validation_status.clone();
        let on_error_toast = on_error_toast.clone();
        spawn_local(async move {
            match validate_yaml(&api.client, yaml_payload).await {
                Ok(result) => {
                    let issues = result
                        .issues
                        .iter()
                        .map(|issue| {
                            let disposition = if issue.blocking {
                                "blocking"
                            } else {
                                "mapping"
                            };
                            format!("{}:{}:{disposition}", issue.pointer, issue.code)
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    validation_status.set(Some(format!(
                        "valid={} version={} profiles={} issues={}",
                        result.valid, result.version, result.profile_count, issues
                    )));
                }
                Err(error) => on_error_toast.emit(error),
            }
        });
    })
}

fn build_apply_callback(
    api: Option<ApiCtx>,
    yaml_input: UseStateHandle<String>,
    toasts: ToastCallbacks,
    on_refresh: Callback<()>,
) -> Callback<MouseEvent> {
    Callback::from(move |_| {
        let Some(api) = api_context(api.clone(), &toasts.error) else {
            return;
        };
        let yaml_payload = (*yaml_input).clone();
        let toasts = toasts.clone();
        let on_refresh = on_refresh.clone();
        spawn_local(async move {
            match apply_yaml(&api.client, yaml_payload).await {
                Ok(result) => {
                    toasts.success.emit(format!(
                        "Media YAML applied ({} profiles)",
                        result.media_profile_public_ids.len()
                    ));
                    on_refresh.emit(());
                }
                Err(error) => toasts.error.emit(error),
            }
        });
    })
}

fn build_save_target_callback(
    api: Option<ApiCtx>,
    state: UseStateHandle<MediaViewState>,
    busy: UseStateHandle<bool>,
    form: TargetCatalogFormHandles,
    toasts: ToastCallbacks,
) -> Callback<MouseEvent> {
    Callback::from(move |_| {
        let Some(api) = api_context(api.clone(), &toasts.error) else {
            return;
        };
        let Some(version) = emit_parse_error(
            parse_catalog_version(&form.version, "Target version"),
            &toasts.error,
        ) else {
            return;
        };
        let Some(audio_channels) = emit_parse_error(
            parse_optional_positive_i32(&form.audio_channels, "Audio channels"),
            &toasts.error,
        ) else {
            return;
        };
        let request = MediaCompatibilityTargetUpsertRequest {
            compatibility_target_key: (*form.key).clone(),
            version,
            display_name: (*form.display_name).clone(),
            video_codec: (*form.video_codec).clone(),
            audio_codec: (*form.audio_codec).clone(),
            audio_channels,
            audio_channel_layout: empty_string_to_none((*form.audio_channel_layout).as_str()),
            subtitle_policy: (*form.subtitle_policy).clone(),
        };
        let state = state.clone();
        let busy = busy.clone();
        let toasts = toasts.clone();
        busy.set(true);
        spawn_local(async move {
            match upsert_compatibility_target(&api.client, &request).await {
                Ok(target) => {
                    let target_key = target.compatibility_target_key.clone();
                    let refreshed_targets = fetch_compatibility_targets(&api.client).await;
                    let mut next = (*state).clone();
                    match refreshed_targets {
                        Ok(response) => {
                            next.compatibility_targets = response.targets;
                        }
                        Err(error) => {
                            toasts.error.emit(format!(
                                "Saved target {target_key}, but target refresh failed: {error}"
                            ));
                        }
                    }
                    upsert_target_state(&mut next.compatibility_targets, target);
                    state.set(next);
                    toasts.success.emit(format!("Saved target {target_key}"));
                }
                Err(error) => toasts.error.emit(error),
            }
            busy.set(false);
        });
    })
}

fn build_save_policy_callback(
    api: Option<ApiCtx>,
    state: UseStateHandle<MediaViewState>,
    busy: UseStateHandle<bool>,
    form: PolicyCatalogFormHandles,
    toasts: ToastCallbacks,
) -> Callback<MouseEvent> {
    Callback::from(move |_| {
        let Some(api) = api_context(api.clone(), &toasts.error) else {
            return;
        };
        let Some(version) = emit_parse_error(
            parse_catalog_version(&form.version, "Policy version"),
            &toasts.error,
        ) else {
            return;
        };
        let Some(verification_duration_tolerance_millis) = emit_parse_error(
            parse_bounded_nonnegative_i64(
                &form.verification_duration_tolerance_millis,
                "Duration tolerance",
                60_000,
            ),
            &toasts.error,
        ) else {
            return;
        };
        let request = MediaPolicyUpsertRequest {
            output: form.output.request(),
            policy_key: (*form.key).clone(),
            version,
            display_name: (*form.display_name).clone(),
            video_intent: (*form.video_intent).clone(),
            verification_strictness: (*form.verification_strictness).clone(),
            verification_duration_tolerance_millis,
            verification_mux_validation: (*form.verification_mux_validation).into(),
            verification_decode_all_streams: (*form.verification_decode_all_streams).into(),
            verification_keyframe_seek: (*form.verification_keyframe_seek).into(),
            verification_playback_probe: (*form.verification_playback_probe).into(),
        };
        let state = state.clone();
        let busy = busy.clone();
        let toasts = toasts.clone();
        busy.set(true);
        spawn_local(async move {
            match upsert_policy(&api.client, &request).await {
                Ok(policy) => {
                    let policy_key = policy.policy_key.clone();
                    let refreshed_policies = fetch_policies(&api.client).await;
                    let mut next = (*state).clone();
                    match refreshed_policies {
                        Ok(response) => {
                            next.policies = response.policies;
                        }
                        Err(error) => {
                            toasts.error.emit(format!(
                                "Saved policy {policy_key}, but policy refresh failed: {error}"
                            ));
                        }
                    }
                    upsert_policy_state(&mut next.policies, policy);
                    state.set(next);
                    toasts.success.emit(format!("Saved policy {policy_key}"));
                }
                Err(error) => toasts.error.emit(error),
            }
            busy.set(false);
        });
    })
}

fn upsert_target_state(
    targets: &mut Vec<MediaCompatibilityTargetResponse>,
    target: MediaCompatibilityTargetResponse,
) {
    if let Some(existing) = targets
        .iter_mut()
        .find(|candidate| candidate.compatibility_target_key == target.compatibility_target_key)
    {
        *existing = target;
    } else {
        targets.push(target);
    }
    targets.sort_by(|left, right| {
        left.compatibility_target_key
            .cmp(&right.compatibility_target_key)
    });
}

fn upsert_policy_state(policies: &mut Vec<MediaPolicyResponse>, policy: MediaPolicyResponse) {
    if let Some(existing) = policies
        .iter_mut()
        .find(|candidate| candidate.policy_key == policy.policy_key)
    {
        *existing = policy;
    } else {
        policies.push(policy);
    }
    policies.sort_by(|left, right| left.policy_key.cmp(&right.policy_key));
}

fn build_job_diagnostics_toggle_callback(
    api: Option<ApiCtx>,
    diagnostics: JobDiagnosticsHandles,
    on_error_toast: Callback<String>,
) -> Callback<uuid::Uuid> {
    let JobDiagnosticsHandles {
        job_diagnostics,
        job_diagnostics_ref,
        opened_job_diagnostics,
        opened_job_diagnostics_ref,
        active_diagnostics_requests,
    } = diagnostics;
    Callback::from(move |media_job_public_id| {
        let (next_opened, disclosure_was_open) = {
            let mut opened = opened_job_diagnostics_ref.borrow_mut();
            let disclosure_was_open = !opened.insert(media_job_public_id);
            if disclosure_was_open {
                opened.remove(&media_job_public_id);
            }
            (opened.clone(), disclosure_was_open)
        };
        if disclosure_was_open {
            opened_job_diagnostics.set(next_opened);
            active_diagnostics_requests
                .borrow_mut()
                .remove(&media_job_public_id);
            let next_job_diagnostics = {
                let mut diagnostics = job_diagnostics_ref.borrow_mut();
                if matches!(
                    diagnostics.get(&media_job_public_id),
                    Some(MediaJobDiagnosticsState::Loading { .. })
                ) {
                    diagnostics.remove(&media_job_public_id);
                }
                diagnostics.clone()
            };
            if next_job_diagnostics != *job_diagnostics {
                job_diagnostics.set(next_job_diagnostics);
            }
            return;
        }
        opened_job_diagnostics.set(next_opened);

        if matches!(
            job_diagnostics_ref.borrow().get(&media_job_public_id),
            Some(MediaJobDiagnosticsState::Loading { .. } | MediaJobDiagnosticsState::Loaded(_))
        ) {
            return;
        }

        let Some(api) = api_context(api.clone(), &on_error_toast) else {
            return;
        };
        let request_id = uuid::Uuid::new_v4();
        active_diagnostics_requests
            .borrow_mut()
            .insert(media_job_public_id, request_id);
        let next_job_diagnostics = {
            let mut diagnostics = job_diagnostics_ref.borrow_mut();
            diagnostics.insert(
                media_job_public_id,
                MediaJobDiagnosticsState::Loading { request_id },
            );
            diagnostics.clone()
        };
        job_diagnostics.set(next_job_diagnostics);

        let job_diagnostics = job_diagnostics.clone();
        let job_diagnostics_ref = job_diagnostics_ref.clone();
        let active_diagnostics_requests = active_diagnostics_requests.clone();
        spawn_local(async move {
            let result = fetch_job_diagnostics(&api.client, media_job_public_id).await;
            let active_request_id = active_diagnostics_requests
                .borrow()
                .get(&media_job_public_id)
                .copied();
            let request_is_current = is_current_diagnostics_request(active_request_id, request_id);
            if !request_is_current {
                return;
            }
            active_diagnostics_requests
                .borrow_mut()
                .remove(&media_job_public_id);

            let load_state = match result {
                Ok(diagnostics) => MediaJobDiagnosticsState::Loaded(diagnostics),
                Err(error) => MediaJobDiagnosticsState::Failed(error),
            };
            let next_job_diagnostics = {
                let mut diagnostics = job_diagnostics_ref.borrow_mut();
                diagnostics.insert(media_job_public_id, load_state);
                diagnostics.clone()
            };
            job_diagnostics.set(next_job_diagnostics);
        });
    })
}

fn render_job_diagnostics_content(load_state: Option<&MediaJobDiagnosticsState>) -> Html {
    match load_state {
        Some(MediaJobDiagnosticsState::Loading { .. }) => html! {
            <div class="collapse-content text-sm opacity-70" data-testid="media-job-diagnostics-loading">
                {"Loading diagnostics..."}
            </div>
        },
        Some(MediaJobDiagnosticsState::Failed(error)) => html! {
            <div class="collapse-content text-sm text-error" role="alert" data-testid="media-job-diagnostics-error">
                {format!("Diagnostics unavailable: {error}")}
            </div>
        },
        Some(MediaJobDiagnosticsState::Loaded(diagnostics)) => html! {
            <div class="collapse-content space-y-3">
                <div class="text-xs opacity-70" data-testid="media-job-diagnostics-summary">
                    {summarize_media_job_diagnostics(diagnostics)}
                </div>
                <div class="grid gap-3 md:grid-cols-3 xl:grid-cols-6">
                    <div>
                        <h3 class="text-sm font-semibold">{"Operations"}</h3>
                        <ul class="space-y-1">{for diagnostics.operations.iter().map(render_job_operation)}</ul>
                    </div>
                    <div>
                        <h3 class="text-sm font-semibold">{"Violations"}</h3>
                        <ul class="space-y-1">{for diagnostics.violations.iter().map(render_job_violation)}</ul>
                    </div>
                    <div>
                        <h3 class="text-sm font-semibold">{"Plan reasons"}</h3>
                        <ul class="space-y-1">{for diagnostics.plan_reasons.iter().map(render_job_plan_reason)}</ul>
                    </div>
                    <div>
                        <h3 class="text-sm font-semibold">{"Verification"}</h3>
                        <ul class="space-y-1">{for diagnostics.verification_checks.iter().map(render_job_verification_check)}</ul>
                    </div>
                    <div>
                        <h3 class="text-sm font-semibold">{"Artifacts"}</h3>
                        <ul class="space-y-1">{for diagnostics.artifacts.iter().map(render_job_artifact)}</ul>
                    </div>
                    <div>
                        <h3 class="text-sm font-semibold">{"Audit facts"}</h3>
                        <ul class="space-y-1">{for diagnostics.compact_audits.iter().map(render_job_compact_audit)}</ul>
                    </div>
                </div>
            </div>
        },
        None => html! {},
    }
}

fn render_job_operation(row: &MediaJobOperationResponse) -> Html {
    let stream = row
        .stream_id
        .map(|stream_id| format!(" stream={stream_id}"))
        .unwrap_or_default();
    let args = [
        row.arg_1.as_deref(),
        row.arg_2.as_deref(),
        row.arg_3.as_deref(),
        row.arg_4.as_deref(),
        row.arg_5.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");
    html! {
        <li class="break-all">
            {format!("#{} {}{} {} {}", row.operation_index, row.operation_kind, stream, row.command_bin, args)}
        </li>
    }
}

fn render_job_violation(row: &MediaJobViolationResponse) -> Html {
    let stream = row
        .stream_id
        .map(|stream_id| format!(" stream={stream_id}"))
        .unwrap_or_default();
    html! {
        <li class="break-all">
            {format!("#{} {} {}{}", row.violation_index, row.severity, row.violation_kind, stream)}
        </li>
    }
}

fn render_job_plan_reason(row: &MediaJobPlanReasonResponse) -> Html {
    let candidate = row
        .candidate_index
        .map(|candidate_index| format!(" candidate={candidate_index}"))
        .unwrap_or_default();
    let selected = if row.selected { "selected" } else { "rejected" };
    html! {
        <li class="break-all">
            {format!("#{} {}{} {} - {}", row.reason_index, selected, candidate, row.reason_code, row.reason_text)}
        </li>
    }
}

fn render_job_verification_check(row: &MediaJobVerificationCheckResponse) -> Html {
    let expected = row
        .expected_value
        .as_deref()
        .map(|value| format!(" expected={value}"))
        .unwrap_or_default();
    let actual = row
        .actual_value
        .as_deref()
        .map(|value| format!(" actual={value}"))
        .unwrap_or_default();
    let details = row
        .details_text
        .as_deref()
        .map(|value| format!(" - {value}"))
        .unwrap_or_default();
    html! {
        <li class="break-all">
            {format!("#{} {} {}{}{}{}", row.check_index, row.check_status, row.check_kind, expected, actual, details)}
        </li>
    }
}

fn render_job_artifact(row: &MediaJobArtifactResponse) -> Html {
    let size = row
        .size_bytes
        .map(|size_bytes| format!(" size={size_bytes}"))
        .unwrap_or_default();
    let content_type = row
        .content_type
        .as_deref()
        .map(|value| format!(" type={value}"))
        .unwrap_or_default();
    html! {
        <li class="break-all">
            {format!("#{} {}{}{} {}", row.artifact_index, row.artifact_kind, size, content_type, row.artifact_path)}
        </li>
    }
}

fn render_job_compact_audit(row: &MediaJobCompactAuditResponse) -> Html {
    html! {
        <li class="break-all">
            {format!("#{} {} - {}", row.audit_index, row.fact_kind, row.fact_text)}
        </li>
    }
}
