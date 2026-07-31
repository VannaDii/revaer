# Media HDR10 mastering display geometry

- Status: Accepted
- Date: 2026-08-01
- Context:
  - HDR10 verification already required BT.2020 color signaling, mastering-display side data, content-light side data, complete scalar payloads, and basic numeric sanity.
  - The mastering-display validator still accepted impossible chromaticity payloads when each individual coordinate was positive and less than or equal to one.
  - A graph-compatible output could therefore pass `hdr_format = hdr10` verification while carrying invalid CIE xy geometry such as `x + y > 1`, collapsed primaries, or a white point outside the declared mastering display primary triangle.
- Decision:
  - Parse mastering display red, green, blue, and white-point coordinates as typed CIE xy points.
  - Reject any point whose coordinates are not positive or whose `x + y` exceeds the valid chromaticity plane.
  - Reject degenerate primary triangles and reject white points outside the declared primary triangle.
  - Alternatives considered:
    - Continue relying on scalar bounds only: rejected because scalar bounds do not prove a valid color volume.
    - Require exact BT.2020 or P3-D65 mastering primaries now: rejected because HDR10 mastering-display metadata reports the mastering display color volume, and real HDR10 assets commonly use P3-D65 primaries inside a BT.2020 container.
- Consequences:
  - Positive outcomes:
    - HDR10 candidate and final verification now fails closed for impossible mastering-display chromaticity geometry.
    - The verifier moves from scalar sanity toward geometric color-volume conformance without adding unsupported exact mastering-target semantics.
  - Risks or trade-offs:
    - Assets with malformed mastering-display metadata now fail explicit HDR10 target verification even when FFprobe exposes all scalar fields.
- Follow-up:
  - Model exact authored mastering-display and content-light target values before requiring equality to operator-selected HDR metadata.
  - Add explicit contracts for HDR10+, Dolby Vision, HLG, and other HDR formats before admitting those labels.

## Task Record

- Motivation:
  - Move the media service closer to production correctness by closing a concrete false-success path in the existing HDR10 verifier.
- Design notes:
  - Kept the current `hdr10` target contract limited to probeable HDR10 evidence.
  - Added geometry validation to the existing mastering-display payload validator rather than widening the desired-target schema.
  - Preserved common P3-D65 mastering-display metadata by validating geometric consistency rather than exact primary labels.
- Test coverage summary:
  - Added focused unit coverage for impossible chromaticity sums, degenerate primary triangles, and white points outside the primary triangle.
  - Reran `cargo --config 'build.rustflags=["-Dwarnings"]' test -p revaer-app hdr10_constraint --all-features`.
- Observability updates:
  - No new metric or log labels were added.
  - Failures continue to surface through the existing `hdr_format=hdr10` video-constraint verification mismatch.
- Status-doc validation:
  - Updated `docs/adr/318-media-transcoding-foundation.md`, `docs/adr/index.md`, and `docs/SUMMARY.md`.
- Risk and rollback plan:
  - Risk: malformed HDR10 files that previously passed with scalar-only metadata now fail verification.
  - Roll back by removing the chromaticity geometry predicates while keeping descriptor, payload, and luminance checks intact.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `.github/instructions/devops.instructions.md`.
  - Reviewed `docs/adr/318-media-transcoding-foundation.md` for the current media-transcoding gap statement.
  - No policy relaxation or stale instruction contradiction was introduced.
