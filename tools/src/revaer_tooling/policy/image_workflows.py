"""Keep image verification reachable and publication bound to authorization.

The predicates below preserve the existing media workflow. Every publication
step consumes the independently resolved image digest, and failed vulnerability
checks still retain their SARIF evidence. PR runs without supplied publication
authorization build and scan locally instead.
"""

from collections.abc import Mapping

from ..images.model import IMAGE_MATRIX
from ..json_data import JsonObject
from .contracts import Contract, runs, workflow
from .formats import Document
from .workflows import mapping, needs

PUBLISH = "inputs.final_image_compliance_bundle != ''"
VERIFY = "inputs.final_image_compliance_bundle == ''"
REACHABLE = "inputs.final_image_compliance_bundle != '' || github.event_name == 'pull_request'"
BUILT_REFERENCE = "${{ steps.build_image.outputs.image_reference }}"
LOCAL_REFERENCE = "${{ steps.verify_image.outputs.image_tag }}"


def image_workflow_findings(documents: Mapping[str, Document], matrix: JsonObject) -> list[str]:
    policy = workflow(documents, "build-images")
    policy.require(
        matrix == IMAGE_MATRIX, "image matrix must define the audited amd64 and arm64 jobs"
    )
    call = mapping(mapping(mapping(policy.document.get("on")).get("workflow_call")).get("inputs"))
    policy.require(
        mapping(call.get("final_image_compliance_bundle"))
        == {
            "description": "Final-image compliance bundle path required before publishing images.",
            "required": "false",
            "type": "string",
            "default": "",
        },
        "image publication must retain its optional compliance authorization input",
    )
    build = policy.job("build")
    policy.require(
        build.get("if") == REACHABLE and build.get("timeout-minutes") == "120",
        "both image architectures must remain reachable with a bounded timeout",
    )
    policy.require(
        build.get("strategy") == {"fail-fast": "false", "matrix": "${{ fromJSON(inputs.matrix) }}"},
        "both image architectures must complete without fail-fast cancellation",
    )
    _conditional(policy, build, "Build & Push Image", PUBLISH, "image-build-push")
    _conditional(policy, build, "Build Image (verification only)", VERIFY, "image-build-verify")
    policy.require(
        policy.step(build, "Log in to GHCR").get("if") == PUBLISH,
        "registry login must require publication authorization",
    )
    supplied = _conditional(
        policy,
        build,
        "Validate supplied compliance authorization against built digest",
        PUBLISH,
        "image-compliance-validate",
        "$FINAL_IMAGE_COMPLIANCE_BUNDLE",
        "$IMAGE_REFERENCE",
    )
    policy.require(
        mapping(supplied.get("env"))
        == {
            "FINAL_IMAGE_COMPLIANCE_BUNDLE": "${{ inputs.final_image_compliance_bundle }}",
            "IMAGE_REFERENCE": BUILT_REFERENCE,
        },
        "supplied compliance authorization must bind to the built digest",
    )
    _scans(policy, build)
    _evidence(policy, build)
    _manifest(policy)
    _helm(policy)
    pr = workflow(documents, "pr")
    caller = pr.job("build-pr-images")
    pr.require(
        caller.get("uses") == "./.github/workflows/build-images.yml",
        "PR image contexts must use the checked-in image workflow",
    )
    pr.require(
        needs(caller)
        == (
            "ui-e2e",
            "feature-matrix",
            "native-it",
            "media-conversion",
            "coverage",
            "supply-chain",
            "load-matrix",
        ),
        "PR images must wait for UI, feature, native, media, coverage, "
        "supply-chain, and matrix gates",
    )
    pr.require(
        caller.get("if") == "github.event.pull_request.head.repo.fork == false",
        "PR image jobs must run on every same-repository PR",
    )
    pr.require(
        caller.get("with")
        == {
            "matrix": "${{ needs.load-matrix.outputs.matrix }}",
            "image_name": "revaer",
            "version_tag": (
                "pr-${{ github.event.pull_request.number }}-"
                "${{ needs.load-matrix.outputs.short_sha }}"
            ),
            "alias_tag": "pr-${{ github.event.pull_request.number }}",
            "include_sha_tag": "false",
            "publish_dev_helm": "true",
            "pr_number": "${{ github.event.pull_request.number }}",
            "checkout_ref": "${{ github.event.pull_request.head.sha }}",
        },
        "PR image caller must preserve source, tags, and dev Helm handling",
    )
    return [*policy.failures, *pr.failures]


def _conditional(
    policy: Contract, job: Document, name: str, condition: str, *command: str
) -> Document:
    step = policy.step(job, name)
    policy.require(
        step.get("if") == condition and runs(step, *command),
        f"{name} must run rv {' '.join(command)} only when {condition}",
    )
    return step


def _scans(policy: Contract, build: Document) -> None:
    for name, command, condition, reference, source in (
        ("Inventory digest-qualified image", "image-inventory", PUBLISH, BUILT_REFERENCE, "remote"),
        ("Scan digest-qualified image", "image-scan", PUBLISH, BUILT_REFERENCE, "remote"),
        ("Scan verification image", "image-scan", VERIFY, LOCAL_REFERENCE, "docker"),
    ):
        step = _conditional(policy, build, name, condition, command)
        environment = mapping(step.get("env"))
        policy.require(
            environment.get("IMAGE_REFERENCE") == reference
            and environment.get("IMAGE_SOURCE") == source
            and environment.get("PLATFORM") == "${{ matrix.platform }}",
            f"{name} must inspect the selected immutable image and platform",
        )
    verifier = policy.step(build, "Verify HIGH and CRITICAL findings")
    policy.require(
        runs(verifier, "trivy-sarif-verify", "trivy-results.sarif") and "if" not in verifier,
        "each architecture must fail on HIGH and CRITICAL findings through rv",
    )
    category = policy.step(build, "Resolve code scanning category")
    policy.require(
        category.get("if") == "always()" and runs(category, "image-scan-category"),
        "SARIF category resolution must run through rv even after a failed vulnerability gate",
    )
    upload = policy.step(build, "Upload scan results")
    policy.require(
        upload.get("if") == "always() && hashFiles('trivy-results.sarif') != ''"
        and mapping(upload.get("with")).get("sarif_file") == "trivy-results.sarif"
        and mapping(upload.get("with")).get("category")
        == "${{ steps.code-scanning-category.outputs.value }}",
        "SARIF must be retained after a failed vulnerability gate",
    )
    trivy = policy.step(build, "Install Trivy")
    policy.require(
        trivy.get("uses") == "aquasecurity/setup-trivy@81e514348e19b6112ce2a7e3ecbafe19c1e1f567"
        and mapping(trivy.get("with")).get("version") == "v0.69.3",
        "image jobs must install the reviewed Trivy version",
    )
    policy.order(
        build,
        (
            "Build & Push Image",
            "Validate supplied compliance authorization against built digest",
            "Build Image (verification only)",
            "Inventory digest-qualified image",
            "Scan digest-qualified image",
            "Scan verification image",
            "Verify HIGH and CRITICAL findings",
            "Upload scan results",
        ),
    )


def _evidence(policy: Contract, build: Document) -> None:
    operations = {
        "Generate digest-bound compliance evidence": (
            "image-compliance-generate",
            "$IMAGE_REFERENCE",
            "image-package-inventory.spdx.json",
            "image-compliance",
        ),
        "Validate digest-bound compliance evidence": (
            "image-compliance-validate",
            "image-compliance/final-image-compliance-bundle.json",
            "$IMAGE_REFERENCE",
        ),
        "Sign image and compliance evidence": ("image-sign-attest",),
        "Verify signed compliance evidence": ("image-attestation-verify",),
    }
    for name, command in operations.items():
        step = _conditional(policy, build, name, PUBLISH, *command)
        policy.require(
            mapping(step.get("env")).get("IMAGE_REFERENCE") == BUILT_REFERENCE,
            f"{name} must consume the independently resolved digest",
        )
    upload = policy.step(build, "Upload compliance evidence")
    policy.require(
        upload.get("if") == PUBLISH
        and mapping(upload.get("with")).get("if-no-files-found") == "error",
        "authorized compliance evidence must be retained fail closed",
    )
    policy.order(build, ("Upload scan results", *operations, "Upload compliance evidence"))


def _manifest(policy: Contract) -> None:
    job = policy.job("create-manifest")
    policy.require(
        needs(job) == ("build",)
        and job.get("if") == REACHABLE
        and job.get("timeout-minutes") == "30",
        "manifest handling must wait for both bounded image analyses",
    )
    _conditional(
        policy, job, "Create and push multi-arch manifest", PUBLISH, "image-manifest-create"
    )
    _conditional(policy, job, "Verify multi-arch manifest inputs", VERIFY, "image-manifest-verify")
    _conditional(policy, job, "Sign multi-arch tags", PUBLISH, "image-manifest-sign")
    policy.order(
        job,
        (
            "Create and push multi-arch manifest",
            "Verify multi-arch manifest inputs",
            "Sign multi-arch tags",
        ),
    )


def _helm(policy: Contract) -> None:
    job = policy.job("publish-dev-helm")
    policy.require(
        needs(job) == ("create-manifest",)
        and job.get("if") == f"inputs.publish_dev_helm && ({REACHABLE})"
        and job.get("timeout-minutes") == "30",
        "dev Helm handling must be bounded and wait for manifest verification",
    )
    versions = policy.step(job, "Resolve Helm versions")
    policy.require(
        versions.get("id") == "chart-versions"
        and runs(versions, "workflow-chart-versions", "--pr-number", "$PR_NUMBER")
        and mapping(versions.get("env")).get("PR_NUMBER") == "${{ inputs.pr_number }}",
        "dev Helm versions must resolve through the shared PR metadata task",
    )
    verifier = policy.step(job, "Verify packaged Helm chart without publishing")
    policy.require(
        runs(verifier, "helm-verify", "$CHART_VERSION", "$APP_VERSION")
        and mapping(verifier.get("env")).get("REVAER_HELM_SIGN") == "0",
        "verification-only Helm output must pass the unsigned package identity check",
    )
    for name, condition in (
        ("Package Helm chart", PUBLISH),
        ("Package Helm chart (verification only)", VERIFY),
        ("Publish dev Helm chart", PUBLISH),
        ("Verify packaged Helm chart without publishing", VERIFY),
    ):
        policy.require(
            policy.step(job, name).get("if") == condition,
            f"{name} must preserve its publication authorization condition",
        )
