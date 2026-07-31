use crate::app::api::ApiCtx;
use crate::features::media::api::{
    apply_yaml, create_profile, export_yaml, fetch_compatibility_targets, fetch_compliance,
    fetch_diagnostics_for_jobs, fetch_jobs_for_profiles, fetch_latest_capability, fetch_policies,
    fetch_profiles, fetch_readiness, patch_profile, preview_discovery, refresh_capability,
    upsert_compatibility_target, upsert_policy, validate_yaml,
};
use crate::features::media::logic::{
    parse_retention_days_input, parse_schedule_interval_input, summarize_media_job_diagnostics,
};
use crate::features::media::state::MediaViewState;
use crate::models::{
    MediaCompatibilityTargetResponse, MediaCompatibilityTargetUpsertRequest,
    MediaDiscoveryPreviewItemResponse, MediaDiscoveryPreviewRequest, MediaJobArtifactResponse,
    MediaJobCompactAuditResponse, MediaJobOperationResponse, MediaJobPlanReasonResponse,
    MediaJobResponse, MediaJobVerificationCheckResponse, MediaJobViolationResponse,
    MediaPolicyResponse, MediaPolicyUpsertRequest, MediaProfilePatchRequest, MediaProfileResponse,
    MediaProfileUpsertRequest,
};
use crate::services::api::ApiClient;
use std::rc::Rc;
use yew::platform::spawn_local;
use yew::prelude::*;

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
    key: UseStateHandle<String>,
    version: UseStateHandle<String>,
    display_name: UseStateHandle<String>,
    video_intent: UseStateHandle<String>,
    unmatched_video_action: UseStateHandle<String>,
    unmatched_audio_action: UseStateHandle<String>,
    unmatched_subtitle_action: UseStateHandle<String>,
    unmatched_attachment_action: UseStateHandle<String>,
    unmatched_data_action: UseStateHandle<String>,
    verification_strictness: UseStateHandle<String>,
    verification_duration_tolerance_millis: UseStateHandle<String>,
    verification_mux_validation: UseStateHandle<bool>,
    verification_decode_all_streams: UseStateHandle<bool>,
    verification_keyframe_seek: UseStateHandle<bool>,
    verification_playback_probe: UseStateHandle<bool>,
}

#[derive(Clone)]
struct ProfileFormHandles {
    profile_key: UseStateHandle<String>,
    source_root: UseStateHandle<String>,
    output_root: UseStateHandle<String>,
    retention_days: UseStateHandle<String>,
    dry_run_only: UseStateHandle<bool>,
    compatibility_target_key: UseStateHandle<String>,
    policy_key: UseStateHandle<String>,
    watcher_enabled: UseStateHandle<bool>,
    schedule_enabled: UseStateHandle<bool>,
    schedule_interval_minutes: UseStateHandle<String>,
    discovery_source_path: UseStateHandle<String>,
}

#[derive(Clone, Copy)]
enum ProfileToggleKind {
    DryRun,
    Watcher,
    Schedule,
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
    let target_catalog_key = use_state(String::new);
    let target_catalog_version = use_state(|| "1".to_string());
    let target_catalog_display_name = use_state(String::new);
    let target_catalog_video_codec = use_state(|| "hevc".to_string());
    let target_catalog_audio_codec = use_state(|| "aac".to_string());
    let target_catalog_audio_channels = use_state(String::new);
    let target_catalog_audio_channel_layout = use_state(String::new);
    let target_catalog_subtitle_policy = use_state(|| "selected".to_string());
    let policy_catalog_key = use_state(String::new);
    let policy_catalog_version = use_state(|| "1".to_string());
    let policy_catalog_display_name = use_state(String::new);
    let policy_catalog_video_intent = use_state(|| "general".to_string());
    let policy_unmatched_video_action = use_state(|| "fail".to_string());
    let policy_unmatched_audio_action = use_state(|| "preserve".to_string());
    let policy_unmatched_subtitle_action = use_state(|| "preserve".to_string());
    let policy_unmatched_attachment_action = use_state(|| "preserve".to_string());
    let policy_unmatched_data_action = use_state(|| "remove".to_string());
    let policy_verification_strictness = use_state(|| "strict".to_string());
    let policy_verification_duration_tolerance_millis = use_state(|| "100".to_string());
    let policy_verification_mux_validation = use_state(|| true);
    let policy_verification_decode_all_streams = use_state(|| true);
    let policy_verification_keyframe_seek = use_state(|| true);
    let policy_verification_playback_probe = use_state(|| true);
    let watcher_enabled = use_state(|| false);
    let schedule_enabled = use_state(|| false);
    let schedule_interval_minutes = use_state(String::new);
    let discovery_source_path = use_state(String::new);

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
        key: policy_catalog_key.clone(),
        version: policy_catalog_version.clone(),
        display_name: policy_catalog_display_name.clone(),
        video_intent: policy_catalog_video_intent.clone(),
        unmatched_video_action: policy_unmatched_video_action.clone(),
        unmatched_audio_action: policy_unmatched_audio_action.clone(),
        unmatched_subtitle_action: policy_unmatched_subtitle_action.clone(),
        unmatched_attachment_action: policy_unmatched_attachment_action.clone(),
        unmatched_data_action: policy_unmatched_data_action.clone(),
        verification_strictness: policy_verification_strictness.clone(),
        verification_duration_tolerance_millis: policy_verification_duration_tolerance_millis
            .clone(),
        verification_mux_validation: policy_verification_mux_validation.clone(),
        verification_decode_all_streams: policy_verification_decode_all_streams.clone(),
        verification_keyframe_seek: policy_verification_keyframe_seek.clone(),
        verification_playback_probe: policy_verification_playback_probe.clone(),
    };
    let profile_form = ProfileFormHandles {
        profile_key: profile_key.clone(),
        source_root: source_root.clone(),
        output_root: output_root.clone(),
        retention_days: retention_days.clone(),
        dry_run_only: dry_run_only.clone(),
        compatibility_target_key: compatibility_target_key.clone(),
        policy_key: policy_key.clone(),
        watcher_enabled: watcher_enabled.clone(),
        schedule_enabled: schedule_enabled.clone(),
        schedule_interval_minutes: schedule_interval_minutes.clone(),
        discovery_source_path: discovery_source_path.clone(),
    };
    let on_refresh = build_refresh_callback(
        api.clone(),
        state.clone(),
        busy.clone(),
        toasts.error.clone(),
    );

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
        Callback::from(move |event: Event| {
            compatibility_target_key.set(
                event
                    .target_unchecked_into::<web_sys::HtmlSelectElement>()
                    .value(),
            );
        })
    };
    let on_policy_key_input = {
        let policy_key = policy_key.clone();
        Callback::from(move |event: Event| {
            policy_key.set(
                event
                    .target_unchecked_into::<web_sys::HtmlSelectElement>()
                    .value(),
            );
        })
    };
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
    let on_policy_unmatched_video_action_change = {
        let value = policy_unmatched_video_action.clone();
        Callback::from(move |event: Event| {
            value.set(
                event
                    .target_unchecked_into::<web_sys::HtmlSelectElement>()
                    .value(),
            );
        })
    };
    let on_policy_unmatched_audio_action_change = {
        let value = policy_unmatched_audio_action.clone();
        Callback::from(move |event: Event| {
            value.set(
                event
                    .target_unchecked_into::<web_sys::HtmlSelectElement>()
                    .value(),
            );
        })
    };
    let on_policy_unmatched_subtitle_action_change = {
        let value = policy_unmatched_subtitle_action.clone();
        Callback::from(move |event: Event| {
            value.set(
                event
                    .target_unchecked_into::<web_sys::HtmlSelectElement>()
                    .value(),
            );
        })
    };
    let on_policy_unmatched_attachment_action_change = {
        let value = policy_unmatched_attachment_action.clone();
        Callback::from(move |event: Event| {
            value.set(
                event
                    .target_unchecked_into::<web_sys::HtmlSelectElement>()
                    .value(),
            );
        })
    };
    let on_policy_unmatched_data_action_change = {
        let value = policy_unmatched_data_action.clone();
        Callback::from(move |event: Event| {
            value.set(
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
    let on_discovery_source_path_input = {
        let discovery_source_path = discovery_source_path.clone();
        Callback::from(move |event: InputEvent| {
            discovery_source_path.set(
                event
                    .target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };

    let on_save_target = build_save_target_callback(
        api.clone(),
        state.clone(),
        busy.clone(),
        target_form.clone(),
        profile_form.compatibility_target_key.clone(),
        toasts.clone(),
    );
    let on_save_policy = build_save_policy_callback(
        api.clone(),
        state.clone(),
        busy.clone(),
        policy_form.clone(),
        profile_form.policy_key.clone(),
        toasts.clone(),
    );
    let on_create_profile = build_create_profile_callback(
        api.clone(),
        profile_form.clone(),
        toasts.clone(),
        on_refresh.clone(),
    );
    let on_toggle_profile_dry_run = build_profile_toggle_callback(
        api.clone(),
        ProfileToggleKind::DryRun,
        toasts.clone(),
        on_refresh.clone(),
    );
    let on_toggle_profile_watcher = build_profile_toggle_callback(
        api.clone(),
        ProfileToggleKind::Watcher,
        toasts.clone(),
        on_refresh.clone(),
    );
    let on_toggle_profile_schedule = build_profile_toggle_callback(
        api.clone(),
        ProfileToggleKind::Schedule,
        toasts.clone(),
        on_refresh.clone(),
    );
    let on_preview_discovery = build_preview_discovery_callback(
        api.clone(),
        state.clone(),
        profile_form.discovery_source_path.clone(),
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

            <div class="grid gap-3 md:grid-cols-4">
                <div class="card bg-base-100 shadow"><div class="card-body"><div class="text-xs uppercase opacity-60">{"Profiles"}</div><div class="text-xl">{state.profiles.len()}</div></div></div>
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
                                    <span class="opacity-70">{format!("v{} {} intent={} unmatched={}/{}/{}/{}/{} verification={} tolerance={}ms mux={} decode={} seek={} playback={}", policy.version, policy.display_name, policy.video_intent, policy.unmatched_video_action, policy.unmatched_audio_action, policy.unmatched_subtitle_action, policy.unmatched_attachment_action, policy.unmatched_data_action, policy.verification_strictness, policy.verification_duration_tolerance_millis, policy.verification_mux_validation, policy.verification_decode_all_streams, policy.verification_keyframe_seek, policy.verification_playback_probe)}</span>
                                </li>
                            })}
                        </ul>
                        <div class="grid gap-2 md:grid-cols-2" data-testid="media-policy-form">
                            <input class="input input-bordered input-sm" placeholder="policy_catalog_key" value={(*policy_catalog_key).clone()} oninput={on_policy_catalog_key_input} />
                            <input class="input input-bordered input-sm" placeholder="policy_version" value={(*policy_catalog_version).clone()} oninput={on_policy_catalog_version_input} />
                            <input class="input input-bordered input-sm" placeholder="policy_display_name" value={(*policy_catalog_display_name).clone()} oninput={on_policy_catalog_display_name_input} />
                            <select class="select select-bordered select-sm" aria-label="policy_video_intent" value={(*policy_catalog_video_intent).clone()} onchange={on_policy_catalog_video_intent_change}>
                                <option value="general">{"General"}</option>
                                <option value="anime">{"Anime"}</option>
                                <option value="archival">{"Archival"}</option>
                            </select>
                            <select class="select select-bordered select-sm" aria-label="policy_unmatched_video_action" value={(*policy_unmatched_video_action).clone()} onchange={on_policy_unmatched_video_action_change}>
                                <option value="fail">{"Fail unmatched video"}</option>
                                <option value="preserve">{"Preserve unmatched video"}</option>
                                <option value="remove">{"Remove unmatched video"}</option>
                            </select>
                            <select class="select select-bordered select-sm" aria-label="policy_unmatched_audio_action" value={(*policy_unmatched_audio_action).clone()} onchange={on_policy_unmatched_audio_action_change}>
                                <option value="preserve">{"Preserve unmatched audio"}</option>
                                <option value="fail">{"Fail unmatched audio"}</option>
                                <option value="remove">{"Remove unmatched audio"}</option>
                            </select>
                            <select class="select select-bordered select-sm" aria-label="policy_unmatched_subtitle_action" value={(*policy_unmatched_subtitle_action).clone()} onchange={on_policy_unmatched_subtitle_action_change}>
                                <option value="preserve">{"Preserve unmatched subtitles"}</option>
                                <option value="fail">{"Fail unmatched subtitles"}</option>
                                <option value="remove">{"Remove unmatched subtitles"}</option>
                            </select>
                            <select class="select select-bordered select-sm" aria-label="policy_unmatched_attachment_action" value={(*policy_unmatched_attachment_action).clone()} onchange={on_policy_unmatched_attachment_action_change}>
                                <option value="preserve">{"Preserve unmatched attachments"}</option>
                                <option value="fail">{"Fail unmatched attachments"}</option>
                                <option value="remove">{"Remove unmatched attachments"}</option>
                            </select>
                            <select class="select select-bordered select-sm" aria-label="policy_unmatched_data_action" value={(*policy_unmatched_data_action).clone()} onchange={on_policy_unmatched_data_action_change}>
                                <option value="remove">{"Remove unmatched data"}</option>
                                <option value="preserve">{"Preserve unmatched data"}</option>
                                <option value="fail">{"Fail unmatched data"}</option>
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
                        <h2 class="text-lg font-semibold">{"Profiles"}</h2>
                        <div class="grid gap-2 md:grid-cols-2" data-testid="media-profile-form">
                            <input class="input input-bordered input-sm" placeholder="profile_key" value={(*profile_key).clone()} oninput={on_profile_key_input} />
                            <input class="input input-bordered input-sm" placeholder="source_root" value={(*source_root).clone()} oninput={on_source_root_input} />
                            <input class="input input-bordered input-sm" placeholder="output_root" value={(*output_root).clone()} oninput={on_output_root_input} />
                            <input class="input input-bordered input-sm" placeholder="retention_days" value={(*retention_days).clone()} oninput={on_retention_days_input} />
                            <select class="select select-bordered select-sm" aria-label="compatibility_target_key" value={(*compatibility_target_key).clone()} onchange={on_compatibility_target_input}>
                                <option value="">{"Default target"}</option>
                                {for state.compatibility_targets.iter().map(|target| html! {
                                    <option value={target.compatibility_target_key.clone()}>
                                        {format!("{} ({}/{})", target.display_name, target.video_codec, target.audio_codec)}
                                    </option>
                                })}
                            </select>
                            <select class="select select-bordered select-sm" aria-label="policy_key" value={(*policy_key).clone()} onchange={on_policy_key_input}>
                                {if state.policies.is_empty() {
                                    html! { <option value="safe_dry_run">{"Safe dry run"}</option> }
                                } else {
                                    html! {}
                                }}
                                {for state.policies.iter().map(|policy| html! {
                                    <option value={policy.policy_key.clone()}>
                                        {format!("{} ({})", policy.display_name, policy.video_intent)}
                                    </option>
                                })}
                            </select>
                            <input class="input input-bordered input-sm" placeholder="schedule_interval_minutes" value={(*schedule_interval_minutes).clone()} oninput={on_schedule_interval_input} />
                            <input class="input input-bordered input-sm" placeholder="discovery_source_path" value={(*discovery_source_path).clone()} oninput={on_discovery_source_path_input} />
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
                                let on_preview_discovery = on_preview_discovery.clone();
                                let media_profile_public_id = row.media_profile_public_id;
                                let preview_source_root = row.source_root.clone();
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
                                        <button
                                            class="btn btn-xs"
                                            onclick={Callback::from(move |_| on_preview_discovery.emit((media_profile_public_id, preview_source_root.clone())))}
                                        >
                                            {"Preview discovery"}
                                        </button>
                                    </li>
                                }
                            })}
                        </ul>
                        {if state.discovery_preview.is_empty() {
                            html! {}
                        } else {
                            html! {
                                <ul class="text-sm space-y-1" data-testid="media-discovery-preview">
                                    {for state.discovery_preview.iter().map(render_discovery_preview)}
                                </ul>
                            }
                        }}
                    </div>
                </div>
                <div class="card bg-base-100 shadow">
                    <div class="card-body gap-2">
                        <h2 class="text-lg font-semibold">{"Recent jobs"}</h2>
                        <ul class="text-sm space-y-1">
                            {for state.jobs.iter().take(10).map(|row| {
                                let diagnostics = state
                                    .job_diagnostics
                                    .get(&row.media_job_public_id)
                                    .cloned()
                                    .unwrap_or_default();
                                html! {
                                    <li>
                                        <details class="collapse collapse-arrow bg-base-200" data-testid="media-job-diagnostics">
                                            <summary class="collapse-title text-sm font-medium">
                                                {format!("{} - {}", row.status, row.source_path)}
                                            </summary>
                                            <div class="collapse-content space-y-3">
                                                <div class="text-xs opacity-70" data-testid="media-job-diagnostics-summary">
                                                    {summarize_media_job_diagnostics(&diagnostics)}
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

fn describe_schedule(enabled: bool, interval: Option<i32>) -> String {
    if !enabled {
        return "off".to_string();
    }
    interval
        .map(|minutes| format!("{minutes}m"))
        .unwrap_or_else(|| "enabled".to_string())
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
    busy: UseStateHandle<bool>,
    on_error_toast: Callback<String>,
) -> Callback<()> {
    Callback::from(move |_| {
        let Some(api) = api_context(api.clone(), &on_error_toast) else {
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
            let compatibility_targets = fetch_compatibility_targets(&api.client).await;
            let policies = fetch_policies(&api.client).await;
            match (
                profiles,
                jobs,
                readiness,
                latest,
                compliance,
                compatibility_targets,
                policies,
            ) {
                (
                    Ok(profiles),
                    Ok(jobs),
                    Ok(readiness),
                    Ok(latest),
                    Ok(compliance),
                    Ok(compatibility_targets),
                    Ok(policies),
                ) => {
                    let current = (*state).clone();
                    let diagnostics_jobs = jobs.jobs.iter().take(10).cloned().collect::<Vec<_>>();
                    state.set(MediaViewState {
                        profiles: profiles.profiles,
                        jobs: jobs.jobs,
                        job_diagnostics: current.job_diagnostics,
                        readiness: Some(readiness),
                        latest_capability: latest.snapshot,
                        compliance: Some(compliance),
                        compatibility_targets: compatibility_targets.targets,
                        policies: policies.policies,
                        yaml_export: current.yaml_export,
                        discovery_preview: current.discovery_preview,
                    });
                    refresh_recent_job_diagnostics(
                        api.client.clone(),
                        state.clone(),
                        diagnostics_jobs,
                        on_error_toast.clone(),
                    );
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
    selected_target: UseStateHandle<String>,
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
        let selected_target = selected_target.clone();
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
                    selected_target.set(target_key.clone());
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
    selected_policy: UseStateHandle<String>,
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
            policy_key: (*form.key).clone(),
            version,
            display_name: (*form.display_name).clone(),
            video_intent: (*form.video_intent).clone(),
            unmatched_video_action: Some((*form.unmatched_video_action).clone()),
            unmatched_audio_action: Some((*form.unmatched_audio_action).clone()),
            unmatched_subtitle_action: Some((*form.unmatched_subtitle_action).clone()),
            unmatched_attachment_action: Some((*form.unmatched_attachment_action).clone()),
            unmatched_data_action: Some((*form.unmatched_data_action).clone()),
            verification_strictness: (*form.verification_strictness).clone(),
            verification_duration_tolerance_millis,
            verification_mux_validation: (*form.verification_mux_validation).into(),
            verification_decode_all_streams: (*form.verification_decode_all_streams).into(),
            verification_keyframe_seek: (*form.verification_keyframe_seek).into(),
            verification_playback_probe: (*form.verification_playback_probe).into(),
        };
        let selected_policy = selected_policy.clone();
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
                    selected_policy.set(policy_key.clone());
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

fn build_create_profile_callback(
    api: Option<ApiCtx>,
    form: ProfileFormHandles,
    toasts: ToastCallbacks,
    on_refresh: Callback<()>,
) -> Callback<MouseEvent> {
    Callback::from(move |_| {
        let Some(api) = api_context(api.clone(), &toasts.error) else {
            return;
        };
        let Some(retention_days) = emit_parse_error(
            parse_retention_days_input(&form.retention_days),
            &toasts.error,
        ) else {
            return;
        };
        let Some(schedule_interval) =
            parse_schedule_interval(&form.schedule_interval_minutes, &toasts.error)
        else {
            return;
        };
        let request = MediaProfileUpsertRequest {
            profile_key: (*form.profile_key).clone(),
            source_root: (*form.source_root).clone(),
            output_root: (*form.output_root).clone(),
            dry_run_only: *form.dry_run_only,
            retention_days,
            compatibility_target_key: empty_string_to_none(
                (*form.compatibility_target_key).as_str(),
            ),
            policy_key: (*form.policy_key).clone(),
            watcher_enabled: *form.watcher_enabled,
            schedule_enabled: *form.schedule_enabled,
            schedule_interval_minutes: schedule_interval,
        };
        let toasts = toasts.clone();
        let on_refresh = on_refresh.clone();
        spawn_local(async move {
            match create_profile(&api.client, &request).await {
                Ok(profile) => {
                    toasts
                        .success
                        .emit(format!("Created profile {}", profile.profile_key));
                    on_refresh.emit(());
                }
                Err(error) => toasts.error.emit(error),
            }
        });
    })
}

fn parse_schedule_interval(
    schedule_interval_minutes: &UseStateHandle<String>,
    on_error: &Callback<String>,
) -> Option<Option<i32>> {
    if schedule_interval_minutes.trim().is_empty() {
        Some(None)
    } else {
        emit_parse_error(
            parse_schedule_interval_input(schedule_interval_minutes),
            on_error,
        )
        .map(Some)
    }
}

fn build_profile_toggle_callback(
    api: Option<ApiCtx>,
    toggle: ProfileToggleKind,
    toasts: ToastCallbacks,
    on_refresh: Callback<()>,
) -> Callback<(uuid::Uuid, bool)> {
    Callback::from(move |(media_profile_public_id, enabled)| {
        let Some(api) = api_context(api.clone(), &toasts.error) else {
            return;
        };
        let request = profile_toggle_request(toggle, enabled);
        let toasts = toasts.clone();
        let on_refresh = on_refresh.clone();
        spawn_local(async move {
            match patch_profile(&api.client, media_profile_public_id, &request).await {
                Ok(profile) => {
                    toasts
                        .success
                        .emit(profile_toggle_message(toggle, &profile));
                    on_refresh.emit(());
                }
                Err(error) => toasts.error.emit(error),
            }
        });
    })
}

fn profile_toggle_request(toggle: ProfileToggleKind, enabled: bool) -> MediaProfilePatchRequest {
    MediaProfilePatchRequest {
        source_root: None,
        output_root: None,
        dry_run_only: matches!(toggle, ProfileToggleKind::DryRun).then_some(enabled),
        retention_days: None,
        compatibility_target_key: None,
        policy_key: None,
        watcher_enabled: matches!(toggle, ProfileToggleKind::Watcher).then_some(enabled),
        schedule_enabled: matches!(toggle, ProfileToggleKind::Schedule).then_some(enabled),
        schedule_interval_minutes: None,
    }
}

fn profile_toggle_message(toggle: ProfileToggleKind, profile: &MediaProfileResponse) -> String {
    match toggle {
        ProfileToggleKind::DryRun => {
            let mode = if profile.dry_run_only {
                "dry-run"
            } else {
                "replace-enabled"
            };
            format!("Profile {} set to {}", profile.profile_key, mode)
        }
        ProfileToggleKind::Watcher => {
            let mode = if profile.watcher_enabled { "on" } else { "off" };
            format!("Profile {} watcher {}", profile.profile_key, mode)
        }
        ProfileToggleKind::Schedule => {
            let mode = if profile.schedule_enabled {
                "on"
            } else {
                "off"
            };
            format!("Profile {} schedule {}", profile.profile_key, mode)
        }
    }
}

fn build_preview_discovery_callback(
    api: Option<ApiCtx>,
    state: UseStateHandle<MediaViewState>,
    discovery_source_path: UseStateHandle<String>,
    toasts: ToastCallbacks,
) -> Callback<(uuid::Uuid, String)> {
    Callback::from(
        move |(media_profile_public_id, default_source_path): (uuid::Uuid, String)| {
            let Some(api) = api_context(api.clone(), &toasts.error) else {
                return;
            };
            let requested_source_path = if discovery_source_path.trim().is_empty() {
                default_source_path
            } else {
                (*discovery_source_path).clone()
            };
            if requested_source_path.trim().is_empty() {
                toasts
                    .error
                    .emit("Discovery source path is required".to_string());
                return;
            }
            let request = MediaDiscoveryPreviewRequest {
                media_profile_public_id,
                source_paths: vec![requested_source_path],
            };
            let state = state.clone();
            let toasts = toasts.clone();
            spawn_local(async move {
                match preview_discovery(&api.client, &request).await {
                    Ok(response) => {
                        let accepted = response
                            .previews
                            .iter()
                            .filter(|preview| preview.accepted)
                            .count();
                        let total = response.previews.len();
                        let mut next = (*state).clone();
                        next.discovery_preview = response.previews;
                        state.set(next);
                        toasts
                            .success
                            .emit(format!("Discovery preview accepted {accepted}/{total}"));
                    }
                    Err(error) => toasts.error.emit(error),
                }
            });
        },
    )
}

fn refresh_recent_job_diagnostics(
    client: Rc<ApiClient>,
    state: UseStateHandle<MediaViewState>,
    jobs: Vec<MediaJobResponse>,
    on_error_toast: Callback<String>,
) {
    spawn_local(async move {
        match fetch_diagnostics_for_jobs(&client, &jobs).await {
            Ok(job_diagnostics) => {
                let mut next = (*state).clone();
                next.job_diagnostics = job_diagnostics;
                state.set(next);
            }
            Err(error) => {
                on_error_toast.emit(format!("Failed to refresh media diagnostics: {error}"));
            }
        }
    });
}

fn render_discovery_preview(row: &MediaDiscoveryPreviewItemResponse) -> Html {
    let outcome = if row.accepted {
        row.output_path
            .as_ref()
            .map(|path| format!("accepted -> {path}"))
            .unwrap_or_else(|| "accepted".to_string())
    } else {
        row.reason.clone().unwrap_or_else(|| "rejected".to_string())
    };
    html! {
        <li class="break-all">
            {format!("{} dry_run={} {}", row.source_path, row.dry_run, outcome)}
        </li>
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
