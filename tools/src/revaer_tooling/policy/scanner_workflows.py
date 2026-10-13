"""Bind scanner execution, credentials, source history and retained evidence.

The same checkout performs coverage and scanning so absolute native paths remain
valid. Exactly one rv task invokes the scanner. Its corresponding report-task ID
then identifies the published result; no second executor can overwrite it.
"""

from collections.abc import Mapping

from .contracts import Contract, runs, workflow
from .formats import Document
from .workflows import mapping, needs, steps

PYTHON_COVERAGE_STEP = "Python tooling coverage"
BROWSER_COVERAGE_STEP = "Run Playwright with browser coverage"
NATIVE_DATABASE_STEP = "Build native compile database"
SCRIPT_COVERAGE_PATH = "coverage/script-coverage.xml"

SCANNER_STEPS = {
    "Coverage": "cov",
    PYTHON_COVERAGE_STEP: "tooling-cov",
    BROWSER_COVERAGE_STEP: "ui-e2e",
    "Merge Python coverage inputs": "python-coverage-merge",
    "Merge browser coverage inputs": "js-coverage-merge",
    NATIVE_DATABASE_STEP: "sonar-compile-db",
    "Verify Sonar analysis inputs": "sonar-verify-inputs",
    "Prepare Sonar source inputs": "sonar-prepare-sources",
    "Prepare exact Sonar SCM context": "sonar-prepare-scm",
    "SonarQube scan": "sonar-scan",
    "Package Sonar analysis evidence": "sonar-package-report",
    "Verify Sonar published result": "sonar-verify-result",
}
COVERAGE_PATHS = (
    "coverage/lcov.info",
    "coverage/rust-sonar-lcov.info",
    "coverage/llvm-cov.txt",
    "coverage/js-lcov.info",
    "coverage/python.xml",
    SCRIPT_COVERAGE_PATH,
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
        distributed = pull_request and "rust-coverage" in mapping(policy.document.get("jobs"))
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
            and mapping(setup[0].get("with")).get("apt-profile")
            == ("base" if distributed else "coverage")
            and mapping(setup[0].get("with")).get("sonar-scanner") == "true",
            "Sonar setup must supply the coverage toolchain and cached scanner analyzers",
        )
        policy.require(
            mapping(job.get("env")).get("REVAER_REQUIRE_NATIVE_COVERAGE") == "1",
            "hosted Linux must require authored native coverage",
        )
        policy.require(
            mapping(job.get("env")).get("PYTHONDONTWRITEBYTECODE") == "1",
            "scanner jobs must set PYTHONDONTWRITEBYTECODE=1 for every CLI step",
        )
        producer_steps = (
            "Coverage",
            PYTHON_COVERAGE_STEP,
            BROWSER_COVERAGE_STEP,
            NATIVE_DATABASE_STEP,
        )
        scanner_steps = {
            key: value
            for key, value in SCANNER_STEPS.items()
            if not distributed or key not in producer_steps
        }
        if distributed:
            _distributed_producers(policy)
        else:
            browser_environment = mapping(policy.step(job, BROWSER_COVERAGE_STEP).get("env"))
            policy.require(
                browser_environment.get("E2E_BROWSER_COVERAGE") == "1",
                "Sonar browser execution must collect precise Chromium coverage",
            )
        for step_name, command in scanner_steps.items():
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
        policy.order(job, tuple(scanner_steps))
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


def _distributed_producers(policy: Contract) -> None:
    scanner = policy.job("coverage")
    producers = ("rust-coverage", "tooling-coverage", "api-e2e", "ui-e2e")
    policy.require(
        set(needs(scanner)) == set(producers), "Sonar must wait for every coverage producer"
    )
    for name in (*producers, "ui-e2e-coverage"):
        job = policy.job(name)
        policy.require("if" not in job, f"coverage producer {name} must run unconditionally")
        checkout = [
            step for step in steps(job) if str(step.get("uses", "")).startswith("actions/checkout@")
        ]
        policy.require(
            len(checkout) == 1
            and mapping(checkout[0].get("with")).get("ref")
            == "${{ github.event.pull_request.head.sha }}",
            f"{name} must execute the same exact submitted head",
        )
    native = policy.job("rust-coverage")
    policy.require(
        mapping(native.get("env")).get("REVAER_REQUIRE_NATIVE_COVERAGE") == "1",
        "native producer must require native coverage",
    )
    for job, commands in (
        (native, {"Coverage": "cov", NATIVE_DATABASE_STEP: "sonar-compile-db"}),
        (
            policy.job("tooling-coverage"),
            {
                PYTHON_COVERAGE_STEP: "tooling-cov",
                "Bootstrap execution coverage": "script-coverage",
            },
        ),
    ):
        for name, command in commands.items():
            step = policy.step(job, name)
            policy.require(
                runs(step, command) and "if" not in step,
                f"producer must run {command} unconditionally",
            )
        policy.order(job, tuple(commands))
    policy.paths(
        policy.step(native, "Upload Rust and native inputs"),
        tuple(
            path
            for path in COVERAGE_PATHS
            if path not in ("coverage/js-lcov.info", "coverage/python.xml", SCRIPT_COVERAGE_PATH)
        ),
    )
    tooling = policy.step(policy.job("tooling-coverage"), "Upload tooling execution inputs")
    policy.paths(tooling, (".coverage",))
    policy.paths(
        policy.step(policy.job("tooling-coverage"), "Upload bootstrap execution input"),
        (SCRIPT_COVERAGE_PATH,),
    )
    policy.require(
        mapping(tooling.get("with")).get("include-hidden-files") == "true",
        "tooling must transport raw coverage databases",
    )
    downloads = {
        "Download Rust and native inputs": {"name": "rust-native-coverage", "path": "coverage"},
        "Download tooling execution inputs": {
            "name": "tooling-coverage",
            "path": "coverage/producers/tooling-coverage",
        },
        "Download API execution inputs": {
            "name": "api-e2e-coverage",
            "path": "coverage/producers/api-e2e-coverage",
        },
        "Download bootstrap execution input": {"name": "bootstrap-coverage", "path": "coverage"},
        "Download browser execution inputs": {
            "pattern": "ui-e2e-*-shard-*",
            "path": "coverage/producers",
        },
    }
    for name, options in downloads.items():
        policy.require(
            policy.step(scanner, name).get("with") == options,
            f"scanner must consume exact producer artifact {name}",
        )
    policy.order(
        scanner,
        (
            *downloads,
            "Merge Python coverage inputs",
            "Merge browser coverage inputs",
            "Verify Sonar analysis inputs",
        ),
    )
