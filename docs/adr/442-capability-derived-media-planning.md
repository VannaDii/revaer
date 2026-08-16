# Capability-derived media planning

- Status: Accepted
- Date: 2026-08-13
- Operator approval: Approved as written by the operator on 2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - Production media preflight loaded an immutable capability snapshot but gave
    the core planner `PlanningConstraints::all_supported()`, allowing candidate
    ranking to assume operations that the loaded toolchain could not execute.
  - The later command builder validated codecs and muxers, but that boundary
    could only reject an already selected plan instead of excluding impossible
    candidates during planning.
  - Planning must use the same inspected source, compiled desired graph, video
    policy, and persisted capability snapshot that execution receives.
- Decision:
  - Build production planning constraints from the exact loaded capability
    snapshot and the bound source/desired stream pairs for the job.
  - Fail preflight closed unless one declared source format has a normalized
    matching demuxer and the desired output container has a matching muxer.
  - Admit each audio, video, or subtitle transcode family only when every bound
    stream of that family has declared codec decode support, a matching decoder,
    declared codec encode support, and an executable encoder for the final
    video policy.
  - Require normalized subtitle capability declarations for both source and
    desired subtitle codecs before admitting subtitle artifact or transcode
    operations.
  - Retain the execution command builder's capability validation as an
    independent fail-closed boundary.
- Consequences:
  - Candidate pruning cannot select operation families absent from the job's
    persisted runtime snapshot.
  - Capability drift produces a deterministic preflight failure or unsupported
    candidate rather than a later command-construction surprise.
  - The operation constraint model remains family-grained, so every bound
    stream in a family must be executable before that family is admitted.
- Follow-up:
  - Apply the operator approval recorded on 2026-08-15 when advancing and
    pushing the implementation.
  - Keep future planner operation kinds mapped to explicit snapshot evidence.
  - Extend production-boundary fixtures when new codecs, containers, or
    subtitle operation families become supported.

## Task Record

- Motivation:
  - Close the v1 production-planning gap where capability data was loaded and
    injected but ignored during candidate selection.
- Design notes:
  - Existing container and declared-stream codec validators are reused so
    planning and command construction share codec/encoder semantics.
  - Existing container and subtitle normalization helpers handle aliases;
    comma-delimited FFmpeg demuxer aliases are evaluated deterministically.
  - Preflight computes the compiled graph and final video policy before
    planning, without changing injected collaborators or adding fallback
    capabilities.
- Test coverage summary:
  - Added focused unit tests proving a complete exact snapshot admits remux,
    audio, video, and subtitle operations.
  - Added focused unit tests proving missing video decode, audio encode, or
    subtitle declarations exclude the affected operations.
  - Added focused unit tests proving missing required demuxer or muxer support
    fails closed with stable error codes.
  - Updated the persisted runtime fixture snapshot to declare the H.264, HEVC,
    MP3, and AAC decode/encode capabilities exercised by the full runtime suite.
  - Ran package formatting, focused unit tests, and package checking for
    `revaer-app`.
- Observability updates:
  - No new metric or event was added. Existing persisted preflight failures and
    compact audit reporting surface the stable capability error codes.
- Status-doc validation:
  - Rechecked `MEDIA_TRANSCODING.md`; the change implements its capability-
    aware preflight requirement without changing the documented scope.
  - No README, roadmap, operator-guide, API, or UI surface changed.
- Risk & rollback plan:
  - The stricter boundary can reject snapshots that previously reached command
    construction, which is intentional fail-closed behavior.
  - Roll back this change to restore late-only command validation if a proven
    snapshot representation defect prevents valid jobs from planning; do not
    fabricate capabilities or restore unconditional planner support.
- Dependency rationale:
  - No dependency was added; the implementation uses existing core
    normalization and runtime capability validators.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`.
  - No policy drift, contradiction, or stale operational reference was found;
    no instruction change is required for this runtime-only behavior.
