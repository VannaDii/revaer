"""Preserve the audited PR contexts and their executable dependency contracts.

The context snapshot was already recorded by the media work. rv changes the
executor, not the externally required job names, shard count, or success rules.
Database rebaseline proof is required when that application contract is present.
"""

from collections.abc import Mapping

from .contracts import Contract, runs, workflow
from .formats import Document
from .workflows import SETUP_PATH, mapping, needs, steps

DOWNLOAD_API_COVERAGE = "Download API E2E coverage"
VERIFY_UI_COVERAGE = "Verify all UI E2E shard coverage inputs"

AUDITED_CONTEXTS = (
    "Check Lint",
    "Check Coverage",
    "Check Formatting",
    "Run Checks",
    "Native Integration Tests",
    "Feature Matrix",
    "UI E2E Coverage",
    "Images / Manifest",
    "Check Instruction Drift",
    "Helm Lint",
    "Images / Helm Chart",
    "Images / Build arm64",
    "Images / Build amd64",
    "Load Matrix",
    "UI E2E (shard 1/3)",
    "UI E2E (shard 2/3)",
    "UI E2E (shard 3/3)",
    "SonarCloud Code Analysis",
    "Media Conversion Fixtures",
    "Supply Chain Checks",
    "Build Release",
)

UI_MATRIX_NAME = (
    "${{ matrix.browser == 'chromium' && 'UI E2E' || format('UI E2E {0}', matrix.browser) }}"
    " (shard ${{ matrix.shard }}/3)"
)


def required_check_findings(
    documents: Mapping[str, Document],
    recorded_contexts: tuple[str, ...],
    database_rebaseline: bool,
    single_init: bool = False,
) -> list[str]:
    policy = workflow(documents, "pr")
    policy.require(recorded_contexts == AUDITED_CONTEXTS, "required PR context snapshot drifted")
    trigger = mapping(policy.document.get("on")).get("pull_request")
    policy.require(
        trigger in ("", {}), "pull_request must have no branch, path, or activity narrowing"
    )
    jobs = mapping(policy.document.get("jobs"))
    emitted: set[str] = set()
    for value in jobs.values():
        job = mapping(value)
        name = job.get("name")
        if not isinstance(name, str) or "uses" in job:
            continue
        if name in ("UI E2E (shard ${{ matrix.shard }}/3)", UI_MATRIX_NAME):
            shards = mapping(mapping(job.get("strategy")).get("matrix")).get("shard")
            policy.require(shards == ["1", "2", "3"], "UI E2E must emit exactly shards 1, 2, and 3")
            if shards == ["1", "2", "3"]:
                emitted.update(f"UI E2E (shard {shard}/3)" for shard in (1, 2, 3))
            if name == UI_MATRIX_NAME:
                strategy = mapping(job.get("strategy"))
                policy.require(
                    mapping(strategy.get("matrix")).get("browser")
                    == ["chromium", "firefox", "webkit"]
                    and strategy.get("max-parallel") == "2"
                    and strategy.get("fail-fast") == "false",
                    "browser matrix must retain three browsers and shards with two concurrent jobs",
                )
        else:
            emitted.add(name)
        if name in AUDITED_CONTEXTS or name in (
            "UI E2E (shard ${{ matrix.shard }}/3)",
            UI_MATRIX_NAME,
        ):
            expected = name == "Supply Chain Checks" and job.get("if") == "always()"
            policy.require(
                "if" not in job or expected,
                f"required PR job {name!r} must not be conditionally skipped",
            )
    images = workflow(documents, "build-images")
    for value in mapping(images.document.get("jobs")).values():
        name = mapping(value).get("name")
        if name == "Build ${{ matrix.name }}":
            # The separate image contract proves both exact matrix entries.
            emitted.update(("Images / Build amd64", "Images / Build arm64"))
        elif isinstance(name, str) and name:
            emitted.add("Images / " + name)
    if any(runs(step, "sonar-scan") for step in steps(mapping(jobs.get("coverage")))):
        emitted.add("SonarCloud Code Analysis")
    for name in AUDITED_CONTEXTS:
        policy.require(name in emitted, f"required PR context is not emitted: {name}")
    _supply_chain(policy)
    _media_conversion(policy)
    _udeps(policy, documents)
    _ui_coverage(policy)
    if single_init:
        feature = policy.job("feature-matrix")
        initialized = policy.step(feature, "Single-init application fixtures")
        policy.require(
            runs(initialized, "ui-e2e-app-test"),
            "Feature Matrix must exercise initialized application fixtures",
        )
        policy.order(
            feature,
            ("Single-init application fixtures", "Feature matrix tests (no default features)"),
        )
        for job_value in mapping(policy.document.get("jobs")).values():
            policy.require(
                not any(runs(step, "db-migrate") for step in steps(mapping(job_value))),
                "Single-init PR jobs must not replay historical migrations",
            )
    elif database_rebaseline:
        feature = policy.job("feature-matrix")
        candidate = policy.step(feature, "Database rebaseline contract")
        policy.require(
            runs(candidate, "db-rebaseline-candidate"),
            "Feature Matrix must execute the database rebaseline proof",
        )
        policy.order(feature, ("Database rebaseline contract", "Run migrations"))
    return [*policy.failures, *images.failures]


def _supply_chain(policy: Contract) -> None:
    job = policy.job("supply-chain")
    policy.require(
        set(needs(job)) == {"audit", "deny", "udeps"},
        "Supply Chain Checks must depend on audit, deny, and udeps",
    )
    policy.require(
        job.get("if") == "always()", "Supply Chain Checks must run even when a dependency fails"
    )
    step = policy.step(job, "Verify supply chain results")
    policy.require(
        runs(step, "verify-supply-chain-results")
        and step.get("env")
        == {
            "AUDIT_RESULT": "${{ needs.audit.result }}",
            "DENY_RESULT": "${{ needs.deny.result }}",
            "UDEPS_RESULT": "${{ needs.udeps.result }}",
        },
        "Supply Chain Checks must verify every dependency result through rv",
    )


def _media_conversion(policy: Contract) -> None:
    """Retain the stack's fixture evidence before cleanup and downstream builds."""
    job = policy.job("media-conversion")
    policy.require(job.get("name") == "Media Conversion Fixtures", "media context name drifted")
    conversion = policy.step(job, "Media conversion integration tests")
    policy.require(
        runs(conversion, "test-media-conversion") and "if" not in conversion,
        "media conversion must execute the canonical task unconditionally",
    )
    upload = policy.step(job, "Upload media conversion report")
    policy.require(
        upload.get("if") == "always()"
        and str(upload.get("uses", "")).startswith("actions/upload-artifact@")
        and mapping(upload.get("with")).get("path") == "target/media-conversion-report.md"
        and mapping(upload.get("with")).get("if-no-files-found") == "error",
        "media conversion must always retain its required nonempty report",
    )
    cleanup = policy.step(job, "Clean media fixtures")
    policy.require(
        cleanup.get("if") == "always()" and runs(cleanup, "clean-test-fixtures"),
        "media fixture cleanup must always run the canonical task",
    )
    policy.order(
        job,
        (
            "Media conversion integration tests",
            "Upload media conversion report",
            "Clean media fixtures",
        ),
    )
    for name in ("build-pr-images", "build-release"):
        policy.require(
            "media-conversion" in needs(policy.job(name)),
            f"{name} must depend on media-conversion",
        )
    policy.require(
        "if" not in policy.job("build-release"),
        "build-release must run on pull requests without a job-level condition",
    )


def _udeps(policy: Contract, documents: Mapping[str, Document]) -> None:
    job = policy.job("udeps")
    policy.require(
        job.get("env")
        == {
            "REVAER_UDEPS_VERSION": "0.1.57",
            "REVAER_UDEPS_TOOLCHAIN": "nightly-2026-06-13",
        },
        "cargo-udeps version and nightly toolchain must remain exact",
    )
    if "rust-coverage" in mapping(policy.document.get("jobs")):
        _shared_udeps_cache(policy, job, documents)
    else:
        cache = policy.step(job, "Restore exact cargo-udeps binary")
        policy.require(
            cache.get("with")
            == {
                "path": "~/.local/revaer/cargo-tools/cargo-udeps",
                "key": (
                    "${{ runner.os }}-cargo-udeps-${{ env.REVAER_UDEPS_VERSION }}-"
                    "${{ env.REVAER_UDEPS_TOOLCHAIN }}"
                ),
            },
            "cargo-udeps cache must include both exact version pins",
        )
    evidence = policy.step(job, "Upload cargo-udeps toolchain evidence")
    policy.require(
        evidence.get("if") == "always()"
        and evidence.get("with")
        == {
            "name": "cargo-udeps-toolchain-evidence",
            "path": "target/udeps-toolchain-evidence.txt",
            "if-no-files-found": "error",
        },
        "cargo-udeps evidence must be retained fail closed",
    )


def _shared_udeps_cache(policy: Contract, job: Document, documents: Mapping[str, Document]) -> None:
    setup = [stage for stage in steps(job) if stage.get("uses") == "./.github/actions/setup-revaer"]
    policy.require(
        len(setup) == 1
        and mapping(setup[0].get("with")).get("cargo-tools") == "cargo-udeps"
        and mapping(setup[0].get("with")).get("compiler-family") == "nightly",
        "cargo-udeps must use its pinned shared installer and nightly compiler cache",
    )
    restored = [
        stage
        for stage in steps(mapping(documents.get(SETUP_PATH, {}).get("runs")))
        if stage.get("id") == "cargo_udeps_cache"
    ]
    policy.require(
        len(restored) == 1
        and str(restored[0].get("uses", "")).startswith("actions/cache/restore@")
        and mapping(restored[0].get("with"))
        == {
            "path": "~/.local/revaer/cargo-tools/cargo-udeps",
            "key": (
                "${{ runner.os }}-${{ runner.arch }}-cargo-tool-cargo-udeps-"
                "${{ hashFiles('rust-toolchain.toml', 'tools/versions.toml') }}"
            ),
        },
        "cargo-udeps shared cache must include both exact version pins and native receipts",
    )


def _ui_coverage(policy: Contract) -> None:
    if "rust-coverage" in mapping(policy.document.get("jobs")):
        _distributed_ui_coverage(policy)
        return
    shards = policy.job("ui-e2e")
    upload = policy.step(shards, "Upload E2E coverage")
    policy.require(
        upload.get("if") == "always()"
        and mapping(upload.get("with"))
        == {
            "name": "ui-e2e-coverage-shard-${{ matrix.shard }}",
            "path": (
                ".test-tmp/e2e/test-results/api-coverage-*.json\n.test-tmp/e2e/test-results/ui-coverage-*.json\n"
            ),
            "if-no-files-found": "error",
        },
        "each UI shard must retain its exact nonempty API and UI coverage artifacts",
    )
    api = policy.job("api-e2e")
    api_upload = policy.step(api, "Upload API E2E coverage")
    policy.require(
        api_upload.get("if") == "always()"
        and api_upload.get("with")
        == {
            "name": "api-e2e-coverage",
            "path": ".test-tmp/e2e/test-results/api-coverage-*.json",
            "if-no-files-found": "error",
        },
        "API E2E coverage must be retained as an exact fail-closed artifact",
    )
    aggregate = policy.job("ui-e2e-coverage")
    policy.require(
        set(needs(aggregate)) == {"api-e2e", "coverage", "feature-matrix", "native-it", "ui-e2e"},
        "UI E2E Coverage must wait for API, every UI shard, feature, native, and coverage gates",
    )
    downloads = {
        DOWNLOAD_API_COVERAGE: "api-e2e-coverage",
        **{
            f"Download E2E coverage shard {shard}": f"ui-e2e-coverage-shard-{shard}"
            for shard in (1, 2, 3)
        },
    }
    for name, artifact in downloads.items():
        policy.require(
            policy.step(aggregate, name).get("with")
            == {
                "name": artifact,
                "path": ".test-tmp/e2e/test-results",
            },
            f"UI E2E Coverage must download the exact artifact {artifact}",
        )
    verification = policy.step(aggregate, VERIFY_UI_COVERAGE)
    policy.require(
        runs(verification, "ui-e2e-shard-coverage"),
        "UI E2E Coverage must prove nonempty records from all three shards",
    )
    policy.order(aggregate, (*downloads, VERIFY_UI_COVERAGE))


def _distributed_ui_coverage(policy: Contract) -> None:
    ui = policy.job("ui-e2e")
    environment = mapping(ui.get("env"))
    policy.require(
        environment.get("E2E_PLAYWRIGHT_PROJECTS") == "ui-${{ matrix.browser }}"
        and environment.get("E2E_BROWSERS") == "${{ matrix.browser }}"
        and environment.get("E2E_PLAYWRIGHT_NO_DEPS") == "1"
        and environment.get("E2E_BROWSER_COVERAGE")
        == "${{ matrix.browser == 'chromium' && '1' || '0' }}",
        "every browser shard must execute its selected browser with Chromium line coverage",
    )
    api = policy.job("api-e2e")
    policy.require(
        mapping(api.get("env")).get("E2E_PLAYWRIGHT_PROJECTS")
        == "api-none-missing-catalog,api-none,api-api-key-missing-catalog,api-api-key",
        "API producer must execute anonymous, authenticated, and both missing-catalog phases",
    )
    for job, name, paths in (
        (
            ui,
            "Upload E2E coverage",
            (
                ".test-tmp/e2e/test-results/python-e2e-summary-shard-*.json",
                ".test-tmp/e2e/test-results/selection-*-shard-*.json",
                "coverage/e2e/ui-*-shard-*/.coverage",
            ),
        ),
        (
            api,
            "Upload API E2E coverage",
            (
                ".test-tmp/e2e/test-results/python-e2e-summary.json",
                ".test-tmp/e2e/test-results/selection-*.json",
                "coverage/e2e/api-none-missing-catalog/.coverage",
                "coverage/e2e/api-none/.coverage",
                "coverage/e2e/api-api-key-missing-catalog/.coverage",
                "coverage/e2e/api-api-key/.coverage",
            ),
        ),
    ):
        upload = policy.step(job, name)
        policy.paths(upload, paths)
        policy.require(
            upload.get("if") == "always()"
            and mapping(upload.get("with")).get("include-hidden-files") == "true",
            "API/browser execution inputs must retain summaries, selections and raw coverage",
        )
    aggregate = policy.job("ui-e2e-coverage")
    policy.require(
        set(needs(aggregate)) == {"api-e2e", "ui-e2e"},
        "UI coverage must wait for both execution producers",
    )
    for name, options in (
        (
            DOWNLOAD_API_COVERAGE,
            {"name": "api-e2e-coverage", "path": "coverage/producers/api-e2e-coverage"},
        ),
        (
            "Download browser execution inputs",
            {"pattern": "ui-e2e-*-shard-*", "path": "coverage/producers"},
        ),
    ):
        policy.require(
            policy.step(aggregate, name).get("with") == options,
            "UI coverage must download current producer inputs without overwriting shards",
        )
    policy.require(
        runs(
            policy.step(aggregate, VERIFY_UI_COVERAGE),
            "ui-e2e-shard-coverage",
        ),
        "every browser partition must be verified before route coverage",
    )
    policy.order(
        aggregate,
        (
            DOWNLOAD_API_COVERAGE,
            "Download browser execution inputs",
            VERIFY_UI_COVERAGE,
            "UI E2E coverage",
        ),
    )
