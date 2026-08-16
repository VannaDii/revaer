# Media HDR10 Side-Data Verification

- Status: Accepted
- Date: 2026-07-22
- Context:
  - Desired target rows can request `hdr10`, and verification already required BT.2020 primaries, SMPTE ST 2084 transfer, and BT.2020 non-constant luminance colorspace.
  - Full inspection also retains FFprobe side-data descriptors, but the HDR10 verification path did not require mastering-display or content-light side-data evidence.
  - That left a graph-compatible and color-compatible output able to pass HDR10 verification while missing probeable HDR metadata that downstream clients commonly depend on.
- Decision:
  - Treat `hdr10` as verified only when the inspected video stream carries both `mastering display metadata` and `content light level metadata` side-data descriptors in addition to the existing color signaling.
  - Keep the check fail-closed through the existing `candidate_video_constraints` and `final_video_constraints` verification records.
  - Keep broader HDR formats and codec-specific metadata semantics out of scope until their desired-target contracts are modeled explicitly.
- Consequences:
  - A candidate can no longer satisfy `hdr10` by color tags alone.
  - Encoders or muxers that omit HDR10 side-data now fail verification for explicitly constrained HDR10 targets.
  - This tightens the implemented HDR10 subset without relaxing any Sonar, coverage, lint, or CI criteria.
- Follow-up:
  - Model richer HDR side-data fields when the desired-target catalog can express exact mastering display and content-light values.
  - Continue rejecting unmodeled chapter, attachment, and arbitrary metadata desired state until complete compile, execution, and verification contracts exist.

## Task Record

- Motivation:
  - Move the media runtime closer to the requested production-complete state by closing a concrete false-success path in the existing HDR10 target contract.
- Design notes:
  - Reuse the normalized side-data type list already retained by complete FFprobe inspection.
  - Preserve the existing `hdr_format=hdr10` expected value and `unverified` actual value for missing HDR evidence, avoiding new unbounded metric or diagnostic labels.
  - Leave non-`hdr10` values rejected by verification rather than inferring unsupported semantics.
- Test coverage summary:
  - Added a media job runtime regression proving a graph-compatible candidate with matching HDR10 color tags but missing HDR side-data fails before replacement.
  - Updated the successful test inspector to model HDR10 side-data for constrained HEVC output so existing happy-path coverage now exercises the stricter evidence requirement.
- Observability updates:
  - No new metrics or log labels were added.
  - Missing HDR10 side-data surfaces through the existing durable video-constraint verification check.
- Status-doc validation:
  - Rechecked ADR 317 and updated its open-gap wording so HDR10 side-data verification is no longer described as entirely open.
- Risk & rollback plan:
  - Risk: some real FFmpeg outputs may carry valid color signaling but omit mastering-display or content-light side-data, causing explicitly constrained HDR10 jobs to fail closed.
  - Roll back by removing the two side-data predicates from `video_hdr_constraint_matches` while keeping color-signaling verification intact.
- Dependency rationale:
  - No new dependencies.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/sonarqube_mcp.instructions.md`, ADR 317, ADR 325, and the Justfile-gated validation policy.
  - No policy drift or criteria relaxation was introduced.
