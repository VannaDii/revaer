"""Cosign owns keyless signatures, certificate identity and attestation checks.

No verification-skip, alternate issuer, or trust override is exposed. A verified
result must come from the reviewed repository's PR or CI workflow identity.
"""

import re
from dataclasses import dataclass
from pathlib import Path

from ..errors import ToolingError
from ..images.model import ghcr_reference
from ..images.validation import PREDICATE_TYPE
from ..process import Completed
from .base import ExternalTool


@dataclass(frozen=True)
class AttestArgs:
    reference: str
    predicate: Path


@dataclass(frozen=True)
class VerifyAttestationArgs:
    reference: str
    repository: str

    def identity(self) -> str:
        ghcr_reference(self.reference)
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", self.repository):
            raise ToolingError("GITHUB_REPOSITORY must be an owner/repository identifier")
        # Repository dots are literal. The old shell expression accidentally
        # treated them, and the .github directory's dot, as regex wildcards.
        return (
            rf"^https://github\.com/{re.escape(self.repository)}/\.github/workflows/(pr|ci)\.yml@"
        )


class Cosign(ExternalTool):
    version_args = ("version",)

    def sign(self, reference: str) -> Completed:
        ghcr_reference(reference)
        return self._invoke(("sign", "--yes", reference))

    def attest(self, args: AttestArgs) -> Completed:
        ghcr_reference(args.reference)
        if args.predicate.is_symlink() or not args.predicate.is_file():
            raise ToolingError("Compliance predicate must be a regular file")
        if not args.predicate.stat().st_size:
            raise ToolingError("Compliance predicate must be nonempty")
        return self._invoke(
            (
                "attest",
                "--yes",
                "--predicate",
                str(args.predicate),
                "--type",
                PREDICATE_TYPE,
                args.reference,
            )
        )

    def verify_attestation(self, args: VerifyAttestationArgs) -> Completed:
        return self._invoke(
            (
                "verify-attestation",
                "--certificate-identity-regexp",
                args.identity(),
                "--certificate-oidc-issuer",
                "https://token.actions.githubusercontent.com",
                "--type",
                PREDICATE_TYPE,
                args.reference,
            ),
            capture=True,
        )
