"""Sonar policy rejects scope reduction, hidden overrides, and missing inputs."""

from pathlib import Path

import pytest
from revaer_tooling.policy.advisories import advisory_findings
from revaer_tooling.policy.sonar import SBOM_PATH, sonar_findings

INVENTORY = (
    ".github/workflows/ci.yml",
    ".sonar-test-scope/.gitkeep",
    "Cargo.toml",
    "charts/revaer/templates/service.yaml",
    "charts/revaer/values.yaml",
    "crates/example/src/lib.rs",
    "docs/adr/decision.md",
    "sonar-project.properties",
    "tools/tests/test_example.py",
    SBOM_PATH,
)
SENTINEL = {".gitkeep": b""}


@pytest.fixture
def sonar() -> str:
    # Start from the actual configuration, retaining every reviewed property.
    # Only the file inventory differs in this small, explicit checkout fixture.
    source = Path(__file__).resolve().parents[2] / "sonar-project.properties"
    changes = {
        "sonar.sources": (
            ".github,Cargo.toml,charts,crates,docs,release,sonar-project.properties,tools"
        ),
        "sonar.lang.patterns.kubernetes": "charts/revaer/templates/service.yaml",
        "sonar.lang.patterns.yaml": ".github/workflows/ci.yml,charts/revaer/values.yaml",
    }
    return (
        "\n".join(
            key + "=" + changes[key] if (key := line.split("=", 1)[0]) in changes else line
            for line in source.read_text().splitlines()
        )
        + "\n"
    )


def change(source: str, key: str, value: str) -> str:
    lines = source.splitlines()
    matches = [index for index, line in enumerate(lines) if line.startswith(key + "=")]
    assert len(matches) == 1, "mutation must change an existing property"
    lines[matches[0]] = f"{key}={value}"
    return "\n".join(lines) + "\n"


def test_reviewed_configuration_and_all_sources_are_accepted(sonar: str) -> None:
    assert not sonar_findings(sonar, INVENTORY, SENTINEL)
    assert sonar_findings(sonar, (*INVENTORY, "new/source.py"), SENTINEL)
    assert sonar_findings(sonar, tuple(path for path in INVENTORY if path != SBOM_PATH), SENTINEL)
    without_inventory = "\n".join(
        line for line in sonar.splitlines() if not line.startswith("sonar.sca.sbomImportPaths=")
    )
    without_inventory = change(
        without_inventory,
        "sonar.sources",
        ".github,Cargo.toml,charts,crates,docs,sonar-project.properties,tools",
    )
    assert not sonar_findings(
        without_inventory, tuple(path for path in INVENTORY if path != SBOM_PATH), SENTINEL
    )


@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("sonar.exclusions", "**/*"),
        ("sonar.coverage.exclusions", "tools/**"),
        ("sonar.inclusions", "crates/**"),
        ("sonar.tests", "tests"),
        ("sonar.sources", "crates"),
        ("sonar.scm.exclusions.disabled", "false"),
        ("sonar.scm.disabled", "true"),
        ("sonar.scm.forceReloadAll", "false"),
        ("sonar.sensor.cache.project.enable", "false"),
        ("sonar.scanner.keepReport", "false"),
        ("sonar.scanner.excludeHiddenFiles", "true"),
        ("sonar.sca.allowManifestFailures", "true"),
        ("sonar.sca.pythonNoResolve", "true"),
        ("sonar.sca.enabled", "false"),
        ("sonar.javascript.detectBundles", "true"),
        ("sonar.javascript.maxFileSize", "10000"),
        ("sonar.filesize.limit", "20"),
        ("sonar.tsql.file.suffixes", ".sql"),
        ("sonar.plsql.file.suffixes", ".sql"),
        ("sonar.python.version", "2.7"),
        ("sonar.python.coverage.reportPaths", "coverage/missing.xml"),
        ("sonar.qualitygate.wait", "false"),
        ("sonar.newCode.referenceBranch", "arbitrary"),
        ("sonar.lang.patterns.yaml", "**/*.yaml"),
        ("sonar.lang.patterns.kubernetes", "charts/revaer/values.yaml"),
        (
            "sonar.lang.patterns.yaml",
            ".github/workflows/ci.yml,.github/workflows/ci.yml,charts/revaer/values.yaml",
        ),
    ],
)
def test_relaxations_and_misclassification_fail(sonar: str, key: str, value: str) -> None:
    errors = sonar_findings(change(sonar, key, value), INVENTORY, SENTINEL)
    assert any(key in error for error in errors)


@pytest.mark.parametrize(
    "extra",
    (
        "sonar.unreviewed=true",
        "sonar.sources=crates",
        "sonar\\.sources=crates",
        "sonar.skip=true",
        "sonar.fixture=value\\",
    ),
)
def test_unknown_and_duplicate_settings_fail(sonar: str, extra: str) -> None:
    assert sonar_findings(sonar + extra + "\n", INVENTORY, SENTINEL)


def test_missing_properties_and_nonempty_sentinel_fail(sonar: str) -> None:
    removed = "\n".join(
        line
        for line in sonar.splitlines()
        if not line.startswith("sonar.python.coverage.reportPaths=")
    )
    assert any("is missing" in error for error in sonar_findings(removed, INVENTORY, SENTINEL))
    for sentinel in ({}, {".gitkeep": b"authored"}, {".gitkeep": b"", "test.py": b""}):
        assert any(
            ".sonar-test-scope" in error for error in sonar_findings(sonar, INVENTORY, sentinel)
        )


def test_advisory_lists_must_exist_and_be_empty() -> None:
    assert not advisory_findings("", "[advisories]\nignore = []\n")
    assert advisory_findings("# no exception\n", "[advisories]\nignore = []\n")
    for manifest in (
        "",
        "advisories = []",
        "[advisories]\n",
        "[advisories]\nignore = ['RUSTSEC-1']",
        "invalid TOML",
    ):
        assert advisory_findings("", manifest)
