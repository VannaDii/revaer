# Helm release annotation rendering slice 9

- Status: Accepted
- Date: 2026-06-01
- Context:
  - `just ci` failed in `just helm-lint` because the Helm packaging script passed multi-line Artifact Hub annotations through `awk -v`.
  - Local awk implementations can reject newline-bearing `-v` values, making chart packaging non-deterministic.
- Decision:
  - Render the chart copy with a shell loop that replaces the release-annotation marker by printing the quoted multi-line annotation block directly.
  - Keep the generated annotation content unchanged.
- Consequences:
  - Positive outcomes:
    - Helm lint and packaging can run on awk variants that reject multi-line variable assignment.
    - Artifact Hub image and prerelease annotations remain generated for release packaging.
  - Risks or trade-offs:
    - The renderer remains intentionally narrow and only replaces the release-annotation marker.
- Follow-up:
  - Re-run `just ci` and `just ui-e2e` after this release-script fix.

## Task Record

- Motivation:
  - Restore deterministic Helm packaging as part of the media transcoding branch validation gate.
- Design notes:
  - Used Bash's existing line-reading primitives instead of adding a new templating dependency.
  - Updated devops instructions because release scripts changed.
- Test coverage summary:
  - Reproduced the failure through `just ci`, which failed at `just helm-lint` with an awk newline error.
  - The fix will be verified by rerunning `just helm-lint` and the full required gates.
- Observability updates:
  - None. This changes release packaging rendering only.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: a malformed chart marker would leave annotations absent from the packaged chart. Roll back by restoring the prior awk renderer if the packaging environment is pinned to a compatible awk, or by replacing the marker through another shell-safe renderer.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/devops.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
