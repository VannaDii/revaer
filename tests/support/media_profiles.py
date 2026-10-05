"""Real native catalog prerequisites shared by media API scenarios."""

from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method
from revaer_tooling.json_data import JsonObject


def native_profile_request(api: ApiClient, suffix: str) -> JsonObject:
    target_key, policy_key = f"target-{suffix}", f"policy-{suffix}"
    target = api.request(
        ApiRequest(
            Method.POST,
            "/v1/media/targets",
            {
                "target_key": target_key,
                "version": 1,
                "display_name": "Profile admission",
                "container_format": "matroska",
                "streams": [
                    {
                        "stream_key": "video-main",
                        "stream_kind": "video",
                        "optional": False,
                        "sort_order": 0,
                        "codec": "hevc",
                        "default_disposition": True,
                        "forced_disposition": False,
                    }
                ],
            },
        )
    )
    assert target.status == 201
    policy = api.request(
        ApiRequest(
            Method.POST,
            "/v1/media/policies",
            {
                "policy_key": policy_key,
                "version": 1,
                "display_name": "Source preservation",
                "video_intent": "general",
                "verification_strictness": "strict",
                "verification_duration_tolerance_millis": 100,
                "verification_mux_validation": True,
                "verification_decode_all_streams": True,
                "verification_keyframe_seek": True,
                "verification_playback_probe": True,
                "output": {
                    "dry_run": True,
                    "replacement_mode": "disabled",
                    "quarantine_enabled": False,
                    "preserve_permissions": True,
                    "preserve_ownership": True,
                },
            },
        )
    )
    assert policy.status == 201
    return {
        "profile_key": f"profile-{suffix}",
        "display_name": "Profile admission",
        "description": "",
        "enabled": True,
        "dry_run_only": True,
        "desired_target_key": target_key,
        "desired_target_version": 1,
        "policy_key": policy_key,
        "policy_version": 1,
        "output_root_key": "ui-source",
        "workspace_root_key": "ui-workspace",
    }
