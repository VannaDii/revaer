# Image evidence

Revaer keeps image production, scanning, evidence validation and publication as
separate operations. A scanner process completing successfully is only one part
of the release gate.

## Commands

| Command | Result |
| --- | --- |
| `rv image-build-push` | Builds one reviewed architecture with provenance/SBOM attestations, then compares Buildx metadata with the independently resolved registry digest before emitting outputs. |
| `rv image-build-verify` | Loads one architecture locally and emits its tag after successful build validation. |
| `rv image-inventory` / `rv image-scan` | Invokes Trivy for an explicit source/platform and retains SPDX or SARIF evidence. Scanner errors fail; findings are evaluated by the next gate. |
| `rv trivy-sarif-verify REPORT` | Checks Trivy SARIF 2.1.0, requires explicit run/result arrays and fails on every reported HIGH/CRITICAL finding. The report remains available on failure. |
| `rv image-compliance-generate REFERENCE INVENTORY OUTPUT` | Creates the existing v2 bundle from the digest-qualified image reference, observed SPDX inventory and committed compliance inputs, then validates it. |
| `rv image-compliance-validate BUNDLE REFERENCE` | Independently checks the image binding, five artifact hashes, package metadata, source evidence and release completion fields. |
| `rv image-sign-attest` / `rv image-attestation-verify` | Validates the complete bundle before keyless signing, then verifies the existing repository/workflow identity, issuer and predicate type through Cosign. |
| `rv image-manifest-create` / `rv image-manifest-sign` | Resolves both source tags before creating the index, requires every destination tag to resolve to the same digest, then signs that immutable digest once. |
| `rv image-manifest-verify` | Checks and reports manifest inputs for verification-only jobs whose images remain local on separate runners. |
| `rv image-scan-category` | Preserves the legacy GitHub code-scanning identity for the reviewed architecture. |
| `rv media-compliance-guardrails` | Checks project/image pins, required runtime packages, exact SPDX declaration versions, source evidence and media labels. Included in `rv policy`. |

Compliance generation and validation do not build, scan, sign or publish an image. The
caller must obtain the inventory from the exact image reference. The generator
must not be used to substitute the declared inventory for an observed inventory
in a publication workflow.

## Inputs and architecture

`images/settings.py` receives workflow environment values at the CLI boundary.
`images/model.py` owns the reviewed amd64/arm64 matrix, Rust targets and name
validation. Task classes coordinate operations; typed Buildx, Trivy and Cosign
adapters construct argument arrays and verify executable availability. No task
interprets shell commands or reads the environment itself.

| Input | Used by |
| --- | --- |
| `IMAGE_NAME`, `VERSION_TAG` | Architecture builds and manifest tasks. |
| `REPOSITORY_OWNER`, optional `ALIAS_TAG`, `INCLUDE_SHA_TAG=true/false` | GHCR publication and manifest inputs; SHA tagging defaults to false. |
| `PLATFORM`, `ARCH_TAG`, `RUST_TARGET` | Architecture builds; the triple must match the reviewed matrix. |
| `BUILDX_BUILDER` | Explicit builder selection; workflows use the setup action's output. Local default: `revaer-builder`. |
| `IMAGE_REFERENCE`, `IMAGE_SOURCE`, `PLATFORM` | Scans; remote references must include the immutable GHCR digest. Inventory requires `remote`. |
| `TRIVY_OUTPUT_PATH` | Optional report path within the checkout; defaults to `image-package-inventory.spdx.json` or `trivy-results.sarif`. |
| `COMPLIANCE_PREDICATE` | Complete v2 bundle used by signing. |
| `GITHUB_REPOSITORY`, `ATTESTATION_OUTPUT` | Certificate identity and retained verified attestation output. |

Build time and Git identity come from the injected context and actual checkout.
Build metadata stays under `artifacts/image-build-ARCH.json`. Report destinations
cannot overwrite tracked source or follow linked paths. Each local report/build
uses an operation lock and invalidates stale evidence before collection.

The workflow retains its publication-authorization input, per-architecture
finding gate, failure-time SARIF upload and downstream manifest/chart ordering.
The certificate expression treats repository dots literally. Manifest signing
uses the resolved digest, so a tag change cannot redirect the signature.
`rv helm-verify VERSION APP_VERSION` also reads an existing package through Helm
and checks its identity and Artifact Hub metadata before it is consumed.

Native interfaces: [Buildx](https://docs.docker.com/reference/cli/docker/buildx/build/),
[manifest creation](https://docs.docker.com/reference/cli/docker/buildx/imagetools/create/),
[Trivy image selection](https://trivy.dev/docs/latest/target/container_image/), and
[Cosign container signing](https://docs.sigstore.dev/cosign/signing/signing_with_containers/).

## Container construction

The [Dockerfile](../../../../Dockerfile) uses the same installed Python package
as local automation. Its stages have distinct responsibilities:

| Stage | Responsibility |
| --- | --- |
| `uv` | Supplies `/uv` from Astral's official digest-pinned image. |
| `tooling` | Runs `uv sync --locked --no-default-groups --managed-python`. uv downloads the pinned interpreter and installs the locked core package. |
| `builder` | Runs `rv container-build` through `uv run --locked --no-default-groups`. Typed Rustup, Cargo and APK adapters prepare the reviewed architecture and compile the application. |
| `runtime` | Copies the binary, docs, configuration and compliance files. BuildKit temporarily mounts the tooling environment while `rv container-runtime` installs exact native packages and prepares the system account/directories. |

The environment and interpreter locations use uv's documented
`UV_PROJECT_ENVIRONMENT` and `UV_PYTHON_INSTALL_DIR` options. `UV_LINK_MODE=copy`
handles the separate cache/filesystem boundary. BuildKit's temporary mounts do
not add those Python files to the resulting application image. The health check
uses Docker's exec form with the existing curl endpoint.

`container-build` and `container-runtime` are internal construction commands;
developers normally use `rv docker-build`. Both require an Alpine build stage
running as root, validate project pins and receive Docker arguments through
immutable `ContainerSettings`. The runtime task additionally verifies the actual
Alpine release before package installation. Native package and account failures
stop the build. Existing runtime accounts must have the expected nonroot group;
repeat preparation reuses them.

Docker contexts omit `.git`. Only these two commands accept an explicit source
archive, verified by Revaer's package marker, Rust 2024 workspace, implementation
entry point and lockfile. Ordinary launcher/CLI dispatch requires a checkout;
an unrelated Git boundary never becomes an archive fallback.

### Pin ownership

[`.github/build-inputs.env`](../../../../.github/build-inputs.env) owns the
reviewed image/frontend digests and exact tool/package versions. `inputs.py`
parses its fixed literal assignments without executing shell content. Duplicate,
unknown, unversioned and executable values fail. The guard compares this manifest
with the Dockerfile, project version pins and committed runtime inventory.

When a pinned Alpine package is replaced upstream, verify the exact replacement
on both architectures, update its declared SPDX version and actual package
checksums, and record the reason in the task ADR. Repository download failures
remain errors; they do not justify unversioned installation or bypassing APK
signature verification. The current OpenSSL/curl pin corrections and retained
upstream evidence are recorded in [ADR 592](../../../../docs/adr/592-python-tooling.md).

Official interfaces: [uv in Docker](https://docs.astral.sh/uv/guides/integration/docker/)
and [BuildKit bind mounts](https://docs.docker.com/reference/dockerfile/#run---mounttypebind).

## Bundle format

`final-image-compliance-bundle.json` retains the existing
`revaer.final-image-compliance.v2` schema and predicate type. It names and hashes:

- `declared-runtime-inventory.spdx.json`: the committed runtime declaration.
- `build-inputs.env`: exact reviewed build inputs.
- `image-package-inventory.spdx.json`: observed packages, identifying the image.
- `source-compliance.json`: source locations and checksums from the declaration.
- `THIRD-PARTY-NOTICES.md`: the committed notices.

Both inventories must contain complete SPDX 2.3 package evidence, including
versions, source locations, asserted licenses and lowercase SHA-256 checksums.
Hash verification does not replace those content checks. Source records must
identify the same image and contain nonempty source/checksum entries. Relative
artifact paths must remain within the bundle, and linked files are rejected.

Generation stages and validates a complete bundle before replacing previous
output. It rejects tracked source, parent traversal and unrelated existing
files. Git HEAD supplies the source revision, even when the workflow event's
SHA names a different commit.

## Tests and scope

The tests use the original clean/vulnerable SARIF fixtures and the repository's
declared inventory as controlled fixture input. They alter metadata, rehash
incomplete package/source records, remove or change every artifact, place
symlinks, and inject copy failures. These tests establish format and failure
behavior; they do not claim a published image is compliant.

The native Buildx/Trivy tests build disposable images, verify source/architecture
labels, and retain real JSON and SARIF findings without changing the selected
builder. Publication tests inject registry and Sigstore outcomes: they prove
coordination and failure behavior, not successful keyless signing or a hosted
workflow run. The converted image workflow passes structural and media-contract
checks.

The unchanged Dockerfile also builds and runs a small Rust 2024 fixture on native
arm64. That check verifies exact runtime package versions, account/file ownership,
exec-form health configuration, the unchanged selected builder, and absence of
Python tooling in the final image. Its complete build log and runtime inventory
are retained under `artifacts/container-dockerfile-proof/`. A preceding transient
DNS failure is retained separately in `artifacts/container-dns-failure.log`.
This proves container construction mechanics; the complete application image
and media integration remain separate acceptance gates. See the
[migration inventory](../../../migration.md).

## Shared PostgreSQL debugger pins

The shared build manifest may also contain the media work's eleven native
PostgreSQL debugger inputs. The block is optional for the foundation and must
be complete when present. Validation requires the debugger image ID, exact
Alpine/APK versions, seven SHA-256 digests and an HTTPS musl source URL without
credentials, query or fragment. Container builds validate this block without
installing debugger tools. Unrecognized fields still fail; this is not a
prefix-based allowance for arbitrary PostgreSQL settings.
