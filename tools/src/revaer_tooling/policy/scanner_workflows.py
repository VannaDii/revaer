"""Bind scanner execution, credentials, source history and retained evidence.

The same checkout performs coverage and scanning so absolute native paths remain
valid. Exactly one rv task invokes the scanner. Its corresponding report-task ID
then identifies the published result; no second executor can overwrite it.
"""

from collections.abc import Mapping

from .contracts import runs, workflow
from .formats import Document
from .workflows import mapping, steps

SCANNER_STEPS = {
    "Coverage": "cov",
    "Python tooling coverage": "tooling-cov",
    "Run Playwright with browser coverage": "ui-e2e",
    "Merge Python coverage inputs": "python-coverage-merge",
    "Merge browser coverage inputs": "js-coverage-merge",
    "Build native compile database": "sonar-compile-db",
    "Verify Sonar analysis inputs": "sonar-verify-inputs",
    "Prepare Sonar source inputs": "sonar-prepare-sources",
    "Prepare exact Sonar SCM context": "sonar-prepare-scm",
    "SonarQube scan": "sonar-scan",
    "Package Sonar analysis evidence": "sonar-package-report",
    "Verify Sonar published result": "sonar-verify-result",
}
COVERAGE_PATHS = (
    "coverage/lcov.info",
    "coverage/llvm-cov.txt",
    "coverage/js-lcov.info",
    "coverage/python.xml",
    "coverage/script-coverage.xml",
    "coverage/compile_commands.json",
    "coverage/cxxbridge/include/rust/cxx.h",
    "coverage/cxxbridge/include/revaer-torrent-libt/src/ffi/bridge.rs.h",
)
SCANNER_PATHS = (
    "artifacts/sonar/scanner.log",
    "artifacts/sonar/scm-evidence.txt",
    ".scannerwork/report-task.txt",
    ".scannerwork/scanner-report.tar.xz",
    "artifacts/sonar/api/*.json",
    "coverage/llvm-cov.txt",
)


def scanner_workflow_findings(documents: Mapping[str, Document]) -> list[str]:
    failures: list[str] = []
    for name, job_name in (("pr", "coverage"), ("sonar", "sonar")):
        policy = workflow(documents, name)
        job = policy.job(job_name)
        pull_request = name == "pr"
        source = (
            "${{ github.event.pull_request.head.sha }}" if pull_request else "${{ github.sha }}"
        )
        checkouts = [
            step
            for step in steps(job)
            if isinstance(step.get("uses"), str)
            and str(step["uses"]).startswith("actions/checkout@")
        ]
        policy.require(
            len(checkouts) == 1
            and mapping(checkouts[0].get("with")) == {"fetch-depth": "0", "ref": source},
            "Sonar checkout must use full history and the exact head SHA",
        )
        setup = [
            step for step in steps(job) if step.get("uses") == "./.github/actions/setup-revaer"
        ]
        policy.require(
            len(setup) == 1
            and mapping(setup[0].get("with")).get("apt-profile") == "coverage"
            and mapping(setup[0].get("with")).get("sonar-scanner") == "true",
            "Sonar setup must supply the coverage toolchain and cached scanner analyzers",
        )
        policy.require(
            mapping(job.get("env")).get("REVAER_REQUIRE_NATIVE_COVERAGE") == "1",
            "hosted Linux must require authored native coverage",
        )
        browser_environment = mapping(
            policy.step(job, "Run Playwright with browser coverage").get("env")
        )
        policy.require(
            browser_environment.get("E2E_BROWSER_COVERAGE") == "1",
            "Sonar browser execution must collect precise Chromium coverage",
        )
        for step_name, command in SCANNER_STEPS.items():
            step = policy.step(job, step_name)
            policy.require(runs(step, command), f"{step_name} must run rv {command}")
            if step_name not in (
                "Package Sonar analysis evidence",
                "Verify Sonar published result",
            ):
                policy.require("if" not in step, f"{step_name} must run unconditionally")
        policy.require(
            sum(runs(step, "sonar-scan") for step in steps(job)) == 1,
            "exactly one authoritative rv sonar-scan invocation is required",
        )
        policy.require(
            policy.step(job, "SonarQube scan").get("env")
            == {"SONAR_TOKEN": "${{ env.SONAR_AUTH_TOKEN }}"},
            "scanner credentials must come from the selected Sonar token",
        )
        scm = {
            "SONAR_BASE_SHA": "${{ github.event.pull_request.base.sha }}"
            if pull_request
            else "${{ github.event.before }}",
            "SONAR_BASE_REF": "${{ github.event.pull_request.base.ref }}"
            if pull_request
            else "main",
            "SONAR_HEAD_SHA": source,
        }
        policy.require(
            policy.step(job, "Prepare exact Sonar SCM context").get("env") == scm,
            "exact Sonar SCM event inputs must match the analyzed source",
        )
        result = policy.step(job, "Verify Sonar published result")
        environment = mapping(result.get("env"))
        policy.require(
            environment.get("SONAR_PROJECT_KEY") == "VannaDii_Revaer",
            "Sonar result project key drifted",
        )
        policy.require(
            environment.get("SONAR_PULL_REQUEST") == "${{ github.event.pull_request.number }}"
            if pull_request
            else "SONAR_PULL_REQUEST" not in environment,
            "PR result verification must query the submitted PR; "
            "main must query the complete backlog",
        )
        policy.order(job, tuple(SCANNER_STEPS))
        coverage = policy.step(job, "Upload coverage artifact")
        evidence = policy.step(job, "Upload Sonar analysis evidence")
        policy.paths(coverage, COVERAGE_PATHS)
        policy.paths(evidence, SCANNER_PATHS)
        policy.require(
            evidence.get("if") == "always()",
            "Sonar evidence must be uploaded even after a failed gate",
        )
        failures.extend(policy.failures)
    return failures
