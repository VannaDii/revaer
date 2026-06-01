# Media runtime compliance artifacts slice 9

- Status: Accepted
- Date: 2026-05-31
- Context:
  - `MEDIA_TRANSCODING.md` slice 9 requires the default container image to ship a redistributable open-source media runtime with concrete source-offer, notice, inventory, and ExifTool exception evidence.
  - The existing Dockerfile only installed the application runtime dependencies, so media capability detection could succeed in local development while the published image lacked the required tools and compliance surfaces.
- Decision:
  - Install Alpine-packaged media tools and codec/font/protocol libraries in the runtime image: FFmpeg, ExifTool, MediaInfo, MKVToolNix, Bento4, libass, x264/x265 runtime libraries, dav1d, Opus, Vorbis, Theora, fontconfig, DejaVu fonts, and GnuTLS.
  - Copy `release/media-compliance` into `/app/compliance` and label the image with the media license mode, source-offer path, notices path, SPDX inventory path, and ExifTool exception path.
  - Add `scripts/media-compliance-guardrails.sh` and wire it into `just lint` through the `policy` recipe so future Docker or release changes cannot silently drop the first-release compliance artifacts or add `--enable-nonfree`.
  - Alternatives considered: building FFmpeg and all codec libraries from source in this slice was deferred because the plan allows packaged redistributable components when exact package inventory/source evidence is captured per published image digest.
- Consequences:
  - Positive outcomes:
    - The default image now contains the runtime tools required by the media adapters already present in this branch.
    - Operators and release automation have stable in-image paths for source offer, notices, inventory/SBOM, and ExifTool exception evidence.
    - The lint gate now fails if the default media runtime drops required packages, labels, or compliance files.
  - Risks or trade-offs:
    - Runtime image size increases materially because the first-release media toolchain is intentionally broad.
    - The static SPDX inventory is a baseline; release automation still must archive exact package versions and package source references for each published image digest.
- Follow-up:
  - Generate per-image package inventory and source-compliance archives during release packaging.
  - Add API/UI About links for the compliance files once the application exposes static compliance metadata.

## Task Record

- Motivation:
  - Continue `MEDIA_TRANSCODING.md` by closing the most visible slice-9 gap between the media runtime plan and the shipped container image.
- Design notes:
  - Used Alpine packages for the first gateable image slice to keep dependency management explicit and avoid adding Rust dependencies.
  - Kept ExifTool bounded by a documented first-release exception artifact rather than silently treating it as a generic runtime utility.
  - Added a repository guardrail instead of relying on documentation because Docker/release drift would otherwise be easy to miss in routine changes.
- Test coverage summary:
  - Added and ran `bash scripts/media-compliance-guardrails.sh`; it failed before Docker/compliance artifacts were added and passes after this slice.
  - Verified Alpine 3.23 can resolve the selected runtime package set with `apk add --simulate`.
- Observability updates:
  - Added OCI labels for media license mode, source offer, notices, inventory/SBOM, and ExifTool exception artifact paths.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: the image grows or an Alpine package name changes. Roll back by reverting this ADR's Dockerfile, release compliance artifacts, guardrail script, and Justfile/devops-instruction changes, then rerun `just lint` and image builds.
- Dependency rationale:
  - No new Rust dependencies were added. Runtime media tools are OS packages required by the media-transcoding plan and are documented in the source-offer and notice artifacts.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/devops.instructions.md`.
  - Drift found: the DevOps instruction file did not mention the new media runtime compliance guardrail.
  - Contradictions or stale references removed: added the guardrail responsibility to the DevOps instruction file.
