"""Real Helm parity for the media chart's immutable compliance bindings.

Schema-only and template-only checks deliberately isolate the two defenses.
Synthetic digests identify no release and no prepared storage. Set
REVAER_TEST_CHART to exercise an integrated checkout instead of the recorded chart.
"""

import copy
import json
import os
import re
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import pytest
import yaml

Values = dict[str, Any]


def documents(source: str) -> list[Values]:
    def unique(node: yaml.Node, seen: set[int]) -> None:
        assert id(node) not in seen, "aliases are not accepted in rendered manifests"
        seen.add(id(node))
        if isinstance(node, yaml.MappingNode):
            keys = []
            for key, value in node.value:
                assert isinstance(key, yaml.ScalarNode)
                keys.append(key.value)
                unique(value, seen)
            assert len(set(keys)) == len(keys), "duplicate YAML keys"
        elif isinstance(node, yaml.SequenceNode):
            for child in node.value:
                unique(child, seen)

    for node in yaml.compose_all(source):
        if node is not None:
            unique(node, set())
    result = list(yaml.safe_load_all(source))
    assert all(item is None or isinstance(item, dict) for item in result)
    return [item for item in result if item is not None]


@dataclass
class Chart:
    source: Path
    schema: Path
    values_file: Path
    defaults: Values
    base: Values

    def run(
        self, values: Values, *, schema: bool = False, skip: bool = False, lint: bool = False
    ) -> subprocess.CompletedProcess[str]:
        self.values_file.write_text(json.dumps(values))
        target = str(self.schema if schema else self.source)
        args = ["lint", target, "--strict"] if lint else ["template", "fixture", target]
        return subprocess.run(
            [
                "helm",
                *args,
                "-f",
                str(self.values_file),
                *(["--skip-schema-validation"] if skip else []),
            ],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )

    def accepted(self, values: Values, **options: bool) -> str:
        result = self.run(values, **options)
        assert result.returncode == 0, result.stdout + result.stderr
        assert not re.search(r"\bwarn(?:ing)?\b", result.stdout + result.stderr, re.I)
        return result.stdout


@pytest.fixture
def chart(tmp_path: Path) -> Chart:
    selected = os.environ.get("REVAER_TEST_CHART")
    source = Path(selected) if selected else Path(__file__).parent / "fixtures/media-chart/revaer"
    schema = tmp_path / "schema-only"
    schema.mkdir()
    for name in ("Chart.yaml", "values.yaml", "values.schema.json"):
        shutil.copy2(source / name, schema / name)
    defaults = yaml.safe_load((source / "values.yaml").read_text())
    base = copy.deepcopy(defaults)
    base["image"].update(digest="sha256:" + "a" * 64, architecture="amd64")
    base["compliance"].update(
        existingClaim="prepared-compliance", manifestDigest="sha256:" + "b" * 64
    )
    base["database"]["existingSecret"] = "fixture-db"
    return Chart(source, schema, tmp_path / "values.json", defaults, base)


def binding(rendered: list[Values], values: Values) -> Values:
    deployments = [item for item in rendered if item["kind"] == "Deployment"]
    assert len(deployments) == 1
    template = deployments[0]["spec"]["template"]
    spec = template["spec"]
    assert len(spec["containers"]) == 1 and "initContainers" not in spec
    app = spec["containers"][0]
    assert app["image"] == values["image"]["repository"] + "@" + values["image"]["digest"]
    assert spec["nodeSelector"] == {
        **values["nodeSelector"],
        "kubernetes.io/arch": values["image"]["architecture"],
    }
    assert [v for v in spec["volumes"] if v["name"] == "compliance"] == [
        {
            "name": "compliance",
            "persistentVolumeClaim": {
                "claimName": values["compliance"]["existingClaim"],
                "readOnly": True,
            },
        }
    ]
    assert [m for m in app["volumeMounts"] if m["name"] == "compliance"] == [
        {
            "name": "compliance",
            "mountPath": "/app/compliance",
            "readOnly": True,
            "subPath": values["image"]["digest"][7:]
            + "/"
            + values["compliance"]["manifestDigest"][7:],
        }
    ]
    annotations = template["metadata"]["annotations"]
    assert annotations["checksum/compliance-manifest"] == values["compliance"]["manifestDigest"]
    assert all(annotations[k] == v for k, v in values["podAnnotations"].items())
    for key, expected in (
        ("imagePullPolicy", values["image"]["pullPolicy"]),
        ("resources", values["resources"]),
        ("securityContext", values["containerSecurityContext"]),
    ):
        assert app[key] == expected
    assert spec["securityContext"] == values["podSecurityContext"]
    for probe in ("readinessProbe", "livenessProbe", "startupProbe"):
        assert (
            app[probe]["exec"]["command"][-1] == "curl -fsS http://127.0.0.1:7070/health >/dev/null"
        )
    assert not any(e["name"].startswith(("REVAER_COMPLIANCE", "REVAER_EXEC_")) for e in app["env"])
    assert not any(
        d["kind"] == "PersistentVolumeClaim"
        and d["metadata"]["name"] == values["compliance"]["existingClaim"]
        for d in rendered
    )
    return deployments[0]


@pytest.mark.parametrize(
    "case",
    [
        "amd64-minimal",
        "arm64-minimal",
        "amd64-selectors",
        "arm64-selectors",
        "image-rollover",
        "manifest-rollover",
        "unqualified",
        "minimum-claim",
        "maximum-claim",
        "custom",
    ],
)
def test_valid_compliance_binding(chart: Chart, case: str) -> None:
    values = chart.base
    if case.startswith(("amd64", "arm64")):
        arch = case.split("-")[0]
        values["image"]["architecture"] = arch
        if case.endswith("selectors"):
            values["nodeSelector"].update(
                {"kubernetes.io/arch": arch, "storage.example.test/class": "fast"}
            )
            values["podAnnotations"]["example.test/note"] = "preserved"
    elif case == "image-rollover":
        values["image"]["digest"] = "sha256:" + "c" * 64
    elif case == "manifest-rollover":
        values["compliance"]["manifestDigest"] = "sha256:" + "d" * 64
    elif case == "unqualified":
        values["image"]["repository"] = "revaer"
    elif case == "minimum-claim":
        values["compliance"]["existingClaim"] = "a"
    elif case == "maximum-claim":
        values["compliance"]["existingClaim"] = ".".join(("a" * 63, "b" * 63, "c" * 63, "d" * 61))
    elif case == "custom":
        values["image"].update(
            repository="registry.example.test:5000/team/revaer", pullPolicy="Always"
        )
        values["imagePullSecrets"] = [{"name": "fixture-registry"}]
        values["database"].update(existingSecret="", url="postgres://db.example.test/revaer")
        values["configPersistence"].update(enabled=True, existingClaim="existing-config")
        values["dataPersistence"]["enabled"] = True
        values["serviceAccount"].update(create=False, name="existing-account")
        values["ingress"]["enabled"] = True
        values.update(
            extraEnv=[{"name": "EXAMPLE_SETTING", "value": "retained"}],
            extraEnvFrom=[{"configMapRef": {"name": "fixture-env"}}],
            podLabels={"example.test/label": "retained"},
            resources={"requests": {"cpu": "100m"}},
            podSecurityContext={"runAsNonRoot": True},
            tolerations=[{"key": "fixture", "operator": "Exists"}],
            affinity={"podAffinity": {"preferredDuringSchedulingIgnoredDuringExecution": []}},
        )
    chart.accepted(values, lint=True)
    rendered = documents(chart.accepted(values))
    deployment = binding(rendered, values)
    assert not chart.accepted(values, schema=True).strip()
    assert rendered == documents(chart.accepted(values, skip=True))
    if case == "custom":
        template = deployment["spec"]["template"]
        spec = template["spec"]
        app = spec["containers"][0]
        for key in ("imagePullSecrets", "affinity", "tolerations"):
            assert spec[key] == values[key]
        assert spec["serviceAccountName"] == "existing-account"
        assert template["metadata"]["labels"]["example.test/label"] == "retained"
        assert values["extraEnv"][0] in app["env"] and app["envFrom"] == values["extraEnvFrom"]
        assert next(v for v in spec["volumes"] if v["name"] == "config")[
            "persistentVolumeClaim"
        ] == {"claimName": "existing-config"}
        assert [m for m in app["volumeMounts"] if m["name"] in ("config", "data")] == [
            {"name": "config", "mountPath": "/config"},
            {"name": "data", "mountPath": "/data"},
        ]
        assert sum(d["kind"] == "PersistentVolumeClaim" for d in rendered) == 1
        assert {"Ingress", "Secret", "Service"} <= {d["kind"] for d in rendered}
        assert re.fullmatch(
            r"[a-f0-9]{64}", template["metadata"]["annotations"]["checksum/database-secret"]
        )


BAD_DIGESTS: list[Any] = [
    "",
    None,
    "latest",
    "a" * 64,
    "sha256:" + "a" * 63,
    "sha256:" + "a" * 65,
    "sha256:" + "A" * 64,
    "SHA256:" + "a" * 64,
    "sha256:" + "g" * 64,
    "sha512:" + "a" * 128,
    " sha256:" + "a" * 64,
    "sha256:" + "a" * 64 + "\n",
    "sha256:" + "a" * 64 + "/../other",
    True,
    42,
    [],
]
BAD_FIELDS: dict[tuple[str, str], list[Any]] = {
    ("image", "digest"): BAD_DIGESTS,
    ("compliance", "manifestDigest"): BAD_DIGESTS,
    ("image", "repository"): [
        "",
        None,
        True,
        42,
        [],
        "revaer:latest",
        "revaer:5000",
        "registry.example.test/team/revaer:mutable",
        "registry.example.test:5000/team/revaer:mutable",
        "registry.example.test/team/revaer@sha256:" + "c" * 64,
        "registry.example.test:tag/team/revaer",
        "registry.example.test/team:tag/revaer",
        "https://registry.example.test/team/revaer",
        "oci://registry.example.test/team/revaer",
        " revaer",
        "revaer ",
        "revaer\n",
        "revaer\t",
        "registry.example.test/\u00a0revaer",
        "revaer\0",
    ],
    ("image", "architecture"): ["", None, "x86_64", "aarch64", "AMD64", "arm64\n", True, []],
    ("image", "tag"): ["latest", "v1.2.3", " ", "\n", None, 42, False],
    ("compliance", "existingClaim"): [
        "",
        None,
        " ",
        " prepared",
        "bad/name",
        "bad..name",
        "Upper",
        "bad_name",
        "-leading",
        "trailing-",
        "x" * 254,
        True,
        42,
        [],
    ],
    ("nodeSelector", "kubernetes.io/arch"): ["arm64"],
    ("podAnnotations", "checksum/compliance-manifest"): [
        "tampered",
        "",
        "sha256:" + "b" * 64,
        None,
    ],
    ("compliance", "enabled"): [False],
    ("compliance", "subPath"): ["latest"],
}


def rejected(chart: Chart, values: Values, diagnostic: str, template_guard: bool = True) -> None:
    for schema, skip in [
        (False, False),
        (True, False),
        *([(False, True)] if template_guard else []),
    ]:
        result = chart.run(values, schema=schema, skip=skip)
        assert result.returncode != 0
        assert re.search(diagnostic, result.stdout + result.stderr, re.S), result.stderr
        assert not result.stdout.strip(), "rejected values emitted installable manifests"


@pytest.mark.parametrize(
    ("path", "value"), [(path, value) for path, values in BAD_FIELDS.items() for value in values]
)
def test_invalid_values_fail_each_independent_guard(
    chart: Chart, path: tuple[str, str], value: Any
) -> None:
    values = chart.base
    values[path[0]][path[1]] = value
    rejected(
        chart,
        values,
        path[0] if path[0] in ("nodeSelector", "podAnnotations") else ".*".join(path),
        path not in (("compliance", "enabled"), ("compliance", "subPath")),
    )


def test_defaults_are_not_installable(chart: Chart) -> None:
    rejected(chart, chart.defaults, "image|compliance")


def test_arm64_conflicting_selector_is_rejected(chart: Chart) -> None:
    chart.base["image"]["architecture"] = "arm64"
    chart.base["nodeSelector"]["kubernetes.io/arch"] = "amd64"
    rejected(chart, chart.base, "nodeSelector")


@pytest.mark.parametrize("source", ["a: 1\na: 2", "a: &value [1]\nb: *value"])
def test_manifest_parser_rejects_duplicate_keys_and_aliases(source: str) -> None:
    with pytest.raises(AssertionError):
        documents(source)
