"""Preserve the original renderer's multiline and unique-marker contracts."""

import pytest
import yaml
from revaer_tooling.errors import ToolingError
from revaer_tooling.tasks.charts import render_annotations

PREFIX = "apiVersion: v2\nname: revaer\nannotations:\n  artifacthub.io/license: MIT\n"
SUFFIX = (
    "  artifacthub.io/links: |\n"
    "    - name: Documentation\n      url: https://example.invalid/docs\n"
)
ANNOTATIONS = {
    "artifacthub.io/prerelease": "true",
    "artifacthub.io/images": (
        "- name: revaer\n  image: ghcr.io/example/revaer:v1.2.3-rc.1\n"
        "  platforms:\n    - linux/amd64\n    - linux/arm64\n"
    ),
    "artifacthub.io/signKey": (
        "fingerprint: '0123456789ABCDEF'\nurl: https://example.invalid/revaer-helm-public.asc\n"
    ),
}


@pytest.mark.parametrize(
    "marker",
    (
        "  # __RELEASE_HELM_ANNOTATIONS__",
        "\t#\t__RELEASE_HELM_ANNOTATIONS__  ",
    ),
)
def test_multiline_annotations_preserve_authored_neighbors(marker: str) -> None:
    rendered = render_annotations(PREFIX + marker + "\n" + SUFFIX, ANNOTATIONS)
    assert rendered.startswith(PREFIX)
    assert rendered.endswith(SUFFIX)
    values = yaml.safe_load(rendered)["annotations"]
    assert {key: values[key] for key in ANNOTATIONS} == ANNOTATIONS
    assert values["artifacthub.io/license"] == "MIT"
    assert yaml.safe_load(values["artifacthub.io/links"])[0]["name"] == "Documentation"


@pytest.mark.parametrize(
    "body",
    (
        "",
        "  value: __RELEASE_HELM_ANNOTATIONS__\n",
        "  # __RELEASE_HELM_ANNOTATIONS__ trailing\n",
        "  # __RELEASE_HELM_ANNOTATIONS__\n  # __RELEASE_HELM_ANNOTATIONS__\n",
    ),
)
def test_missing_embedded_or_duplicate_markers_fail(body: str) -> None:
    with pytest.raises(ToolingError, match="exactly one standalone"):
        render_annotations(PREFIX + body + SUFFIX, ANNOTATIONS)
