"""Catalog-backed editors: controlled transport, draft retention, and layout.

These scenarios preserve the media worktree's UI assertions. They do not claim
that fixture paths have been attested or that a routed write was persisted.
"""

import base64
from urllib.parse import parse_qs, urlsplit

import pytest
from playwright.sync_api import Locator, Page, Route, expect
from revaer_tooling.json_data import JsonObject, array_value, object_value

from tests.conftest import ARTIFACTS
from tests.pages.app_shell import AppShell

PROFILE = "00000000-0000-0000-0000-000000000011"
ASSOCIATION = "00000000-0000-0000-0000-000000000022"
REQUESTED = "/private/catalog-source/library"
CANONICAL = "/private/catalog-canonical/library"


def catalog_route(url: str) -> bool:
    parsed = urlsplit(url)
    return parsed.path == "/v1/media/root-catalog" and parse_qs(parsed.query).get("limit") == [
        "200"
    ]


def profiles_route(url: str) -> bool:
    parsed = urlsplit(url)
    return parsed.path == "/v1/media/profiles" and parse_qs(parsed.query).get("limit") == ["200"]


def catalog() -> JsonObject:
    """Return a fresh single-page, source-only transport fixture."""
    return {
        "format_version": 1,
        "source_state": "ready",
        "attestation_state": "ready",
        "generation": {
            "media_root_catalog_generation_public_id": "00000000-0000-0000-0000-000000000033",
            "generation": "1",
            "source_sha256": "a" * 64,
            "generation_sha256": "b" * 64,
            "slot_count": 1,
            "activated_at": "2026-09-15T00:00:00Z",
            "reconciled_at": "2026-09-15T00:00:00Z",
        },
        "slots": [
            {
                "media_root_catalog_slot_public_id": "00000000-0000-0000-0000-000000000044",
                "logical_key": "library",
                "requested_path": REQUESTED,
                "canonical_path": CANONICAL,
                "filesystem_device": "0000000000000001",
                "filesystem_inode": "0000000000000002",
                "mount_id": "1",
                "filesystem_type": "ext4",
                "read_capable": True,
                "write_capable": True,
                "create_new_capable": True,
                "fsync_capable": True,
                "rename_capable": True,
                "delete_capable": True,
                "capacity_probe_capable": True,
                "durability_class": "disposable",
                "durability_evidence": "none",
                "sole_writer_class": "revaer_exclusive",
                "sole_writer_evidence": "linux_dedicated_service",
                "owner_uid": 1000,
                "owner_gid": 1000,
                "mode_bits": "0700",
                "validated_at": "2026-09-15T00:00:00Z",
                "root_identity_sha256": "c" * 64,
                "allowed_kinds": [
                    {
                        "kind": "source",
                        "binding_ready": True,
                        "destructive_ready": False,
                        "destructive_reason": "media_root_durability_unproven",
                    }
                ],
            }
        ],
    }


@pytest.fixture(autouse=True)
def editor_catalog(page: Page) -> None:
    page.route(catalog_route, lambda route: route.fulfill(json=catalog()))
    page.route(
        profiles_route,
        lambda route: route.fulfill(
            json={
                "profiles": [
                    {
                        "media_profile_public_id": PROFILE,
                        "profile_key": "balanced",
                        "latest_version": 3,
                        "active_version": 3,
                    }
                ]
            }
        ),
    )


def select_source(editor: Locator, key: str) -> None:
    editor.get_by_role("textbox", name="Association key").fill(key)
    editor.get_by_label("Active profile version").select_option(PROFILE)
    editor.get_by_role("combobox", name="Source root", exact=True).select_option("library")


def capture_layout(
    page: Page,
    editor: Locator,
    request: pytest.FixtureRequest,
    name: str,
    *,
    fields: bool = False,
) -> None:
    for width in (1280, 390):
        page.set_viewport_size({"width": width, "height": 844})
        for element in editor.get_by_role("combobox").all() if fields else [editor]:
            bounds = element.bounding_box()
            assert bounds is not None and bounds["x"] + bounds["width"] <= width
        editor.screenshot(
            path=request.node.stash[ARTIFACTS] / f"{name}-{width}.png",
            animations="disabled",
        )


def test_explicit_whole_root_and_create_precondition(
    app: AppShell,
    page: Page,
    request: pytest.FixtureRequest,
) -> None:
    submissions = []

    def submit(route: Route) -> None:
        payload = object_value(route.request.post_data_json)
        submissions.append(payload)
        assert route.request.headers["if-none-match"] == "*"
        assert payload == {
            "association_key": "scan-library",
            "media_profile_public_id": PROFILE,
            "profile_version": 3,
            "source_root_key": "library",
            "root_relative_path": "",
            "manual_enabled": True,
            "watcher_enabled": False,
            "schedule_enabled": False,
        }
        route.fulfill(
            status=201,
            headers={"ETag": '"association-created-v1"', "Access-Control-Expose-Headers": "ETag"},
            json={
                **payload,
                "media_discovery_association_public_id": ASSOCIATION,
                "latest_version": 1,
                "active_version": 1,
            },
        )

    page.route("**/v1/media/discovery-associations", submit)
    app.goto("/media")
    admin = page.get_by_test_id("media-root-catalog")
    admin.get_by_text("library", exact=True).click()
    expect(admin).to_contain_text(REQUESTED)
    expect(admin).to_contain_text(CANONICAL)
    editor = page.get_by_test_id("media-association-editor")
    expect(editor).not_to_contain_text(REQUESTED)
    expect(editor).not_to_contain_text(CANONICAL)
    select_source(editor, "scan-library")
    create = editor.get_by_role("button", name="Create association", exact=True)
    create.click()
    expect(editor.get_by_role("alert")).to_have_text("Select whole-root or relative-prefix scope.")
    assert not submissions
    editor.get_by_label("Whole root", exact=True).check()
    editor.get_by_label("Manual", exact=True).check()
    capture_layout(page, editor, request, "association")
    create.click()
    expect(editor.get_by_role("status")).to_contain_text("active version 1")
    assert len(submissions) == 1
    expect(create).to_be_disabled()


def test_conflict_preserves_prefix_and_modes_without_retry(app: AppShell, page: Page) -> None:
    submissions = []

    def submit(route: Route) -> None:
        payload = object_value(route.request.post_data_json)
        submissions.append(payload)
        assert payload["root_relative_path"] == "Series/Season 1"
        assert payload["watcher_enabled"] is True and payload["schedule_enabled"] is True
        route.fulfill(status=412, json={"title": "Conflict", "status": 412, "detail": REQUESTED})

    page.route("**/v1/media/discovery-associations", submit)
    app.goto("/media")
    editor = page.get_by_test_id("media-association-editor")
    select_source(editor, "scan-series")
    editor.get_by_role("radio", name="Relative prefix", exact=True).check()
    prefix = editor.get_by_role("textbox", name="Relative prefix", exact=True)
    prefix.fill("Series/Season 1")
    for label in ("Watcher", "Schedule"):
        editor.get_by_label(label, exact=True).check()
    editor.get_by_role("button", name="Create association", exact=True).click()
    expect(editor.get_by_role("alert")).to_contain_text("preserved draft")
    expect(editor).not_to_contain_text(REQUESTED)
    expect(prefix).to_have_value("Series/Season 1")
    for label in ("Watcher", "Schedule"):
        expect(editor.get_by_label(label, exact=True)).to_be_checked()
    assert len(submissions) == 1


def test_incomplete_catalog_withholds_choices(app: AppShell, page: Page) -> None:
    incomplete = catalog()
    object_value(incomplete["generation"])["slot_count"] = 2
    page.route(catalog_route, lambda route: route.fulfill(json=incomplete))
    app.goto("/media")
    expect(page.get_by_test_id("media-root-catalog").get_by_role("alert")).to_be_visible()
    editor = page.get_by_test_id("media-association-editor")
    expect(
        editor.get_by_role("combobox", name="Source root", exact=True).locator("option")
    ).to_have_count(1)
    expect(editor.get_by_role("button", name="Create association", exact=True)).to_be_disabled()


def test_profile_roots_preserve_kind_scoped_drafts_after_access_loss(
    app: AppShell,
    page: Page,
    request: pytest.FixtureRequest,
) -> None:
    app.goto("/media")
    editor = page.get_by_test_id("media-profile-root-editor")
    expect(
        editor.get_by_role("combobox", name="Output root", exact=True).locator("option")
    ).to_have_count(1)
    expect(editor.get_by_label("Enabled", exact=True)).not_to_be_checked()
    expect(editor.get_by_label("Dry-run only", exact=True)).to_be_checked()
    for label in ("Desired target version", "Policy version"):
        expect(editor.get_by_label(label, exact=True)).to_have_value("")
    available = catalog()
    object_value(available["generation"])["slot_count"] = 4
    slot = object_value(array_value(available["slots"])[0])
    available["slots"] = [
        {
            **slot,
            "media_root_catalog_slot_public_id": f"00000000-0000-0000-0000-00000000005{index}",
            "logical_key": kind,
            "requested_path": f"{REQUESTED}/{kind}",
            "canonical_path": f"{CANONICAL}/{kind}",
            "filesystem_inode": f"000000000000000{index + 3}",
            "root_identity_sha256": str(index + 1) * 64,
            "allowed_kinds": [
                {
                    "kind": kind,
                    "binding_ready": True,
                    "destructive_ready": False,
                    "destructive_reason": "media_root_durability_unproven",
                }
            ],
        }
        for index, kind in enumerate(("backup", "output", "quarantine", "workspace"))
    ]
    page.route(catalog_route, lambda route: route.fulfill(json=available))
    page.get_by_role("button", name="Reload root catalog").click()
    kinds = ("Output", "Workspace", "Backup", "Quarantine")
    for kind in kinds:
        field = editor.get_by_role("combobox", name=f"{kind} root", exact=True)
        expect(field.locator("option")).to_have_count(2)
        expect(field).to_have_value("")
        field.select_option(kind.lower())
    expect(
        editor.get_by_text("Binding ready. Destructive operations are not ready.", exact=True)
    ).to_have_count(4)
    expect(editor).not_to_contain_text(REQUESTED)
    expect(editor).not_to_contain_text(CANONICAL)
    for label, value in (
        ("Profile key", "library-profile"),
        ("Display name", "Library profile"),
        ("Desired target key", "archive"),
        ("Desired target version", "2"),
        ("Policy key", "preserve"),
        ("Policy version", "3"),
    ):
        editor.get_by_label(label, exact=True).fill(value)
    submissions = []

    def submit(route: Route) -> None:
        if route.request.method != "POST":
            route.continue_()
            return
        payload = object_value(route.request.post_data_json)
        submissions.append(payload)
        assert route.request.headers["if-none-match"] == "*"
        assert payload == {
            "profile_key": "library-profile",
            "display_name": "Library profile",
            "description": "",
            "enabled": False,
            "dry_run_only": True,
            "desired_target_key": "archive",
            "desired_target_version": 2,
            "policy_key": "preserve",
            "policy_version": 3,
            "output_root_key": "output",
            "workspace_root_key": "workspace",
            "backup_root_key": "backup",
            "quarantine_root_key": "quarantine",
        }
        route.fulfill(status=412, json={"title": "Conflict", "status": 412})

    page.route("**/v1/media/profiles", submit)
    editor.get_by_role("button", name="Create profile", exact=True).click()
    expect(editor.get_by_role("alert")).to_contain_text("preserved draft")
    expect(editor.get_by_label("Profile key", exact=True)).to_have_value("library-profile")
    assert len(submissions) == 1
    capture_layout(page, editor, request, "profile-roots", fields=True)
    page.route(
        catalog_route,
        lambda route: route.fulfill(status=403, json={"title": "Denied", "status": 403}),
    )
    page.get_by_role("button", name="Reload root catalog").click()
    expect(page.get_by_test_id("media-root-catalog").get_by_role("alert")).to_have_text(
        "Access to the root catalog was denied."
    )
    for kind in kinds:
        expect(editor.get_by_role("combobox", name=f"{kind} root", exact=True)).to_have_value(
            kind.lower()
        )
    expect(
        editor.get_by_text(
            "Selected key is unavailable for this kind. Unsaved selection retained.", exact=True
        )
    ).to_have_count(4)
    editor.get_by_role("combobox", name="Backup root", exact=True).select_option("")
    expect(editor.get_by_text("No optional root selected.", exact=True)).to_be_visible()


def test_all_pages_required_and_changed_generation_rejected(app: AppShell, page: Page) -> None:
    first, second = catalog(), catalog()
    for value in (first, second):
        object_value(value["generation"])["slot_count"] = 2
    key = b"library"
    cursor = (
        base64.urlsafe_b64encode(
            len(key).to_bytes(2, "big") + key + bytes.fromhex("00000000000000000000000000000044")
        )
        .decode()
        .rstrip("=")
    )
    slot = object_value(array_value(second["slots"])[0])
    slot.update(
        {
            "media_root_catalog_slot_public_id": "00000000-0000-0000-0000-000000000045",
            "logical_key": "z-library",
            "requested_path": REQUESTED + "-two",
            "canonical_path": CANONICAL + "-two",
            "filesystem_inode": "0000000000000003",
            "root_identity_sha256": "d" * 64,
        }
    )
    pages = []

    def respond(route: Route) -> None:
        selected = parse_qs(urlsplit(route.request.url).query).get("cursor")
        pages.append(selected)
        if selected:
            assert selected == [cursor]
            route.fulfill(json=second)
        else:
            route.fulfill(json={**first, "next_cursor": cursor})

    page.route(catalog_route, respond)
    app.goto("/media")
    editor = page.get_by_test_id("media-association-editor")
    choices = editor.get_by_role("combobox", name="Source root", exact=True).locator("option")
    expect(choices).to_have_count(3)
    assert len(pages) == 2
    object_value(second["generation"])["generation"] = "2"
    page.get_by_role("button", name="Reload root catalog").click()
    expect(page.get_by_test_id("media-root-catalog").get_by_role("alert")).to_have_text(
        "The catalog response is inconsistent. Reload before configuring associations."
    )
    expect(choices).to_have_count(1)
    expect(editor.get_by_role("button", name="Create association", exact=True)).to_be_disabled()
