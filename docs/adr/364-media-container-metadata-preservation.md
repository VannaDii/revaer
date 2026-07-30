# Media Container Metadata Preservation

- Status: Accepted
- Date: 2026-07-30
- Context:
  - Full inspection already retains source container metadata, but primary media output construction only explicitly preserved chapters.
  - A graph-compatible candidate could therefore drop source container tags and still pass the existing graph, stream-constraint, and chapter checks.
  - Arbitrary authored metadata rewrite remains unsupported, so this change must preserve discovered source metadata without creating a new edit contract.
- Decision:
  - Capture the inspected source container metadata during worker preflight and carry it through the candidate and final verification context.
  - Emit `-map_metadata 0` for primary media FFmpeg outputs beside the existing chapter-preservation arguments.
  - Add fail-closed source, candidate, and final container-metadata verification checks. Empty source metadata is accepted as absent; non-empty source metadata must be present in the output, while tool-generated extra tags are allowed.
  - Keep arbitrary metadata rewrite operations fail-closed until target schema, conflict behavior, and verification semantics are authored.
- Consequences:
  - Source-level container tags such as title and first-party catalog markers are no longer silently dropped by an otherwise graph-compatible output.
  - Verification audit rows now expose `source_container_metadata`, `candidate_container_metadata`, and `final_container_metadata` check kinds.
  - Outputs may still contain additional FFmpeg-generated tags; verification requires the source metadata subset rather than byte-for-byte tag equality.
- Follow-up:
  - Add a future authored metadata edit contract only with explicit target schema, conflict rules, and source/final verification evidence.

## Task Record

- Motivation:
  - Close the source container-metadata preservation gap without broadening the unsupported arbitrary metadata rewrite surface.
- Design notes:
  - Reused the existing full-inspection `ContainerInspection.metadata` records and normalized metadata comparison helper.
  - Candidate and final checks share the same fail-closed path and telemetry/error code: `media_job_output_container_metadata_mismatch`.
  - Check indexes are additive after the existing chapter checks so previous audit kinds keep their current indexes.
- Test coverage summary:
  - Added runtime tests proving candidate metadata loss fails before replacement and committed metadata loss rolls back to original source bytes.
  - Updated FFmpeg argv tests to require `-map_metadata 0` for remux, staged operation, desired-graph, and sidecar-embed primary media outputs.
- Observability updates:
  - Added persisted verification check kinds for source, candidate, and final container metadata.
  - Reused the existing bounded verification failure metric and `MediaJobVerificationFailed` event.
- Status-doc validation:
  - Updated the consolidated media foundation ADR, ADR index, and documentation summary to reflect source container metadata preservation while leaving authored metadata rewrites open.
- Risk & rollback plan:
  - Roll back by removing the metadata preservation argv flag and the associated preflight context/checks. Existing chapter and graph checks remain independent.
  - If a container/muxer cannot retain a required source tag, the job fails closed instead of replacing the source; operators can choose a different target or approve a future explicit metadata policy.
- Dependency rationale:
  - No new dependency was added; the change uses existing FFmpeg/FFprobe process boundaries and inspection types.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - No policy relaxation, lint suppression, workflow bypass, Sonar narrowing, or dependency exception was introduced.
