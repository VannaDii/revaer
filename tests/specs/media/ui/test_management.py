"""Real media catalog writes through the management page."""

import uuid

import pytest
from playwright.sync_api import Page, Response, expect

from tests.pages.app_shell import AppShell


@pytest.mark.timeout(60)
def test_media_management_catalogs_and_controls(app: AppShell, page: Page) -> None:
    app.goto("/media")
    expect(page.get_by_role("heading", name="Media", exact=True)).to_be_visible()
    refresh = page.get_by_role("button", name="Refresh", exact=True)
    expect(refresh).to_be_visible()
    for label in ("Refresh capability", "Export YAML"):
        expect(page.get_by_role("button", name=label, exact=True)).to_be_visible()
    expect(page.get_by_text("License mode")).to_be_visible()
    expect(refresh).to_be_enabled(timeout=20000)
    compliance = page.get_by_test_id("media-compliance-panel")
    expect(compliance).to_be_visible()
    expect(compliance).to_contain_text("License mode")
    for title, test_id in (
        ("Compatibility targets", "media-target-catalog"),
        ("Policies", "media-policy-catalog"),
    ):
        expect(page.get_by_role("heading", name=title, exact=True)).to_be_visible()
        expect(page.get_by_test_id(test_id)).to_be_attached()
    profile = page.get_by_test_id("media-profile-form")
    expect(profile).to_be_visible()
    for label in ("compatibility_target_key", "policy_key"):
        expect(profile.get_by_label(label)).to_be_visible()
    expect(page.get_by_placeholder("schedule_interval_minutes")).to_be_visible()
    for label in ("Enable watcher", "Enable schedule"):
        expect(profile.get_by_text(label)).to_be_visible()
    expect(profile.get_by_role("button", name="Create profile", exact=True)).to_be_visible()

    suffix = uuid.uuid4().hex
    target_key, policy_key = f"ui-target-{suffix}", f"ui-policy-{suffix}"
    target = page.get_by_test_id("media-target-form")
    for placeholder, value in (
        ("target_key", target_key),
        ("target_version", "1"),
        ("target_display_name", f"UI target {suffix}"),
        ("target_video_codec", "hevc"),
        ("target_audio_codec", "aac"),
        ("target_audio_channels", "2"),
        ("target_audio_channel_layout", "stereo"),
    ):
        target.get_by_placeholder(placeholder).fill(value)
    target.get_by_label("target_subtitle_policy").select_option("selected")

    def target_write(response: Response) -> bool:
        return (
            response.url.endswith("/v1/media/compatibility-targets")
            and response.request.method == "POST"
        )

    with page.expect_response(target_write) as created_target:
        target.get_by_role("button", name="Save target").click()
    assert created_target.value.status == 201
    expect(refresh).to_be_disabled()
    expect(refresh).to_be_enabled(timeout=30000)
    expect(
        page.get_by_test_id("media-target-catalog").get_by_text(target_key, exact=True)
    ).to_be_visible(timeout=10000)

    policy = page.get_by_test_id("media-policy-form")
    for placeholder, value in (
        ("policy_catalog_key", policy_key),
        ("policy_version", "1"),
        ("policy_display_name", f"UI policy {suffix}"),
    ):
        policy.get_by_placeholder(placeholder).fill(value)
    policy.get_by_label("policy_video_intent").select_option("general")

    def policy_write(response: Response) -> bool:
        return response.url.endswith("/v1/media/policies") and response.request.method == "POST"

    with page.expect_response(policy_write) as created_policy:
        policy.get_by_role("button", name="Save policy").click()
    assert created_policy.value.status == 201
    expect(refresh).to_be_disabled()
    expect(refresh).to_be_enabled(timeout=30000)
    expect(
        page.get_by_test_id("media-policy-catalog").get_by_text(policy_key, exact=True)
    ).to_be_visible(timeout=10000)
    for label, value in (("compatibility_target_key", target_key), ("policy_key", policy_key)):
        field = profile.get_by_label(label)
        field.select_option(value)
        expect(field).to_have_value(value)
    expect(page.get_by_role("heading", name="YAML import/export", exact=True)).to_be_visible()
    for label in ("Validate YAML", "Apply YAML"):
        expect(page.get_by_role("button", name=label, exact=True)).to_be_visible()
