"""Enforce the reviewed Sonar scope and analyzer inputs without scanner overrides.

The properties file remains the scanner's sole configuration. This allowlist is
the review gate for that file: unknown keys fail, every required key must exist,
and analysis scope filters must be explicitly empty. It never repairs a failing policy.
"""

from collections.abc import Mapping
from fnmatch import fnmatchcase

from ..errors import ToolingError
from .formats import properties

EMPTY_KEYS = frozenset(
    (
        "sonar.exclusions",
        "sonar.inclusions",
        "sonar.test.exclusions",
        "sonar.test.inclusions",
        "sonar.cpd.exclusions",
        "sonar.issue.ignore.multicriteria",
        "sonar.issue.ignore.allfile",
        "sonar.issue.ignore.block",
        "sonar.issue.enforce.multicriteria",
        "sonar.javascript.exclusions",
        "sonar.sca.exclusions",
    )
)

COVERAGE_EXCLUSIONS = (
    ".github/**",
    "charts/**",
    "docs/**",
    "release/**",
    "scripts/**",
    "test-fixtures/**",
    "tests/**",
    "tools/**",
    "vendor/**",
    "setup.sh",
    "crates/revaer-doc-indexer/**",
    "crates/revaer-test-support/**",
    "crates/revaer-ui/tools/**",
    "crates/revaer-ui/ui_vendor/**",
    "crates/revaer-ui/static/nexus/js/**",
    "**/tests/**",
    "**/tests.rs",
    "**/*_tests.rs",
    "**/*_tests/**",
    "**/build.rs",
)


def production_source(path: str) -> bool:
    """Apply the operator-approved coverage scope without narrowing analysis."""
    return not any(fnmatchcase(path, pattern) for pattern in COVERAGE_EXCLUSIONS)


REQUIRED_VALUES = {
    "sonar.coverage.exclusions": ",".join(COVERAGE_EXCLUSIONS),
    "sonar.projectKey": "VannaDii_Revaer",
    "sonar.organization": "vannadii",
    "sonar.sourceEncoding": "UTF-8",
    "sonar.verbose": "true",
    "sonar.scanner.excludeHiddenFiles": "false",
    "sonar.text.activate": "true",
    "sonar.text.inclusions.activate": "true",
    "sonar.text.inclusions": "**/*",
    "sonar.html.file.suffixes": ".html",
    "sonar.tsql.file.suffixes": ".tsql",
    "sonar.plsql.file.suffixes": ".plsql",
    "sonar.plsql.defaultSchema": "public",
    "sonar.featureflag.cloud-security-enable-generic-yaml-and-json-analyzer": "true",
    "sonar.yaml.activate": "true",
    "sonar.json.activate": "true",
    "sonar.scm.exclusions.disabled": "true",
    "sonar.scm.disabled": "false",
    "sonar.scm.provider": "git",
    "sonar.scm.forceReloadAll": "true",
    "sonar.sensor.cache.project.enable": "true",
    "sonar.scanner.keepReport": "true",
    "sonar.tests": ".sonar-test-scope",
    "sonar.filesize.limit": "100",
    "sonar.javascript.maxFileSize": "100000",
    "sonar.javascript.detectBundles": "false",
    "sonar.sca.enabled": "true",
    "sonar.sca.allowManifestFailures": "false",
    "sonar.sca.goNoResolve": "false",
    "sonar.sca.mavenNoResolve": "false",
    "sonar.sca.gradleNoResolve": "false",
    "sonar.sca.pythonNoResolve": "false",
    "sonar.sca.npmNoResolve": "false",
    "sonar.sca.nugetNoResolve": "false",
    "sonar.sca.cfamily": "true",
    "sonar.rust.clippy.enabled": "true",
    "sonar.rust.lcov.reportPaths": "coverage/lcov.info",
    "sonar.javascript.lcov.reportPaths": "coverage/js-lcov.info",
    "sonar.python.coverage.reportPaths": "coverage/python.xml",
    "sonar.python.version": "3.13",
    "sonar.coverageReportPaths": "coverage/script-coverage.xml",
    "sonar.cfamily.compile-commands": "coverage/compile_commands.json",
    "sonar.cfamily.llvm-cov.reportPath": "coverage/llvm-cov.txt",
    "sonar.newCode.referenceBranch": "main",
    "sonar.qualitygate.wait": "true",
    "sonar.qualitygate.timeout": "600",
}
SBOM_KEY = "sonar.sca.sbomImportPaths"
SBOM_PATH = "release/media-compliance/media-runtime-inventory.spdx.json"


def sonar_findings(
    source: str,
    inventory: tuple[str, ...],
    sentinel: Mapping[str, bytes],
) -> list[str]:
    """Check reviewed values and the complete, current source inventory.

    ``inventory`` contains existing tracked and nonignored untracked files. The
    empty sentinel disables Sonar's test-path heuristic; it may never contain
    authored code. Both inputs are captured by task wiring, outside this validator.
    """
    try:
        entries = properties(source, "sonar-project.properties")
    except ToolingError as error:
        return [str(error)]
    configured = {entry.key: entry.value for entry in entries}
    required = {**dict.fromkeys(EMPTY_KEYS, ""), **REQUIRED_VALUES}
    if SBOM_PATH in inventory:
        required[SBOM_KEY] = SBOM_PATH
    required["sonar.sources"] = ",".join(
        sorted({path.split("/", 1)[0] for path in inventory} - {".sonar-test-scope"})
    )
    # These two lists classify Kubernetes templates and ordinary YAML. Their
    # exact file membership depends on the checkout and is validated below.
    classifications = {"sonar.lang.patterns.yaml", "sonar.lang.patterns.kubernetes"}
    allowed = required.keys() | classifications
    failures = [f"unrecognized Sonar property {key}" for key in sorted(configured.keys() - allowed)]
    failures.extend(
        f"required Sonar property {key} is missing" for key in sorted(allowed - configured.keys())
    )
    failures.extend(
        f"{key} must equal {expected!r}"
        for key, expected in required.items()
        if key in configured and configured[key] != expected
    )
    if sentinel != {".gitkeep": b""}:
        failures.append(".sonar-test-scope must contain only an empty .gitkeep")
    failures.extend(_classifications(configured, inventory))
    return failures


def _classifications(configured: dict[str, str], inventory: tuple[str, ...]) -> list[str]:
    yaml_paths = {path for path in inventory if path.endswith((".yml", ".yaml"))}
    kubernetes = {path for path in yaml_paths if path.startswith("charts/revaer/templates/")}
    expected = {
        "sonar.lang.patterns.kubernetes": kubernetes,
        "sonar.lang.patterns.yaml": yaml_paths - kubernetes,
    }
    failures: list[str] = []
    for key, paths in expected.items():
        if key not in configured:
            continue
        actual = configured[key].split(",") if configured[key] else []
        if len(actual) != len(set(actual)) or set(actual) != paths:
            failures.append(f"{key} must enumerate exactly: {','.join(sorted(paths))}")
    return failures
