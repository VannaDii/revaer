"""Independent image evidence gates, shared by local and workflow invocations."""

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..images.compliance import BUNDLE, generate_bundle, validate_bundle
from ..images.policy import media_compliance_findings
from ..images.validation import read_document, verify_sarif
from .base import Task


class MediaComplianceGuardrails(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        failures = media_compliance_findings(context)
        if failures:
            raise ToolingError("Media compliance guardrails failed:\n" + "\n".join(failures))
        return TaskResult("Runtime package, source-offer and image pin guardrails passed")


class TrivySarifVerify(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        if context.options.report is None:
            raise ToolingError("Select the Trivy SARIF report to verify")
        path = context.root / context.options.report
        verify_sarif(read_document(context.fs, path))
        return TaskResult("Trivy SARIF contains no HIGH or CRITICAL findings")


class ImageComplianceGenerate(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        options = context.options
        if options.inventory is None or options.output is None:
            raise ToolingError("Select the package inventory and compliance output directory")
        artifacts = context.root / "artifacts"
        context.fs.mkdir(artifacts)
        output = context.root / options.output
        with context.fs.lock(artifacts / ".image-compliance.lock"):
            generate_bundle(
                context, options.image_reference, context.root / options.inventory, output
            )
        return TaskResult(f"Digest-bound compliance bundle verified: {output / BUNDLE}")


class ImageComplianceValidate(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        if context.options.bundle is None:
            raise ToolingError("Select the compliance bundle to verify")
        validate_bundle(
            context.fs, context.root / context.options.bundle, context.options.image_reference
        )
        return TaskResult("Image digest, artifact hashes, SPDX and source compliance verified")
