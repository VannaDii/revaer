# Media job profile root validation slice 10/13

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Media profiles represent the current path-to-profile association, but manual job creation accepted arbitrary source and output paths.
  - Dry-run jobs still need to be bounded by the selected profile roots so planning and diagnostics cannot reference unrelated paths.
- Decision:
  - Always load the owning media profile before creating a job.
  - Reject source paths outside the profile source root and provided output paths outside the profile output root.
  - Preserve existing dry-run override and capability checks after path validation.
- Consequences:
  - Positive outcomes:
    - Job creation now enforces the profile/root association for both dry-run and non-dry-run jobs.
    - Sibling path prefixes such as `/input/app-media-other` are not accepted for `/input/app-media`.
  - Risks or trade-offs:
    - Existing callers that queued jobs with paths outside the selected profile now receive a validation error.
- Follow-up:
  - Route manual discovery through the same job-creation path once discovery execution is added.
  - Add UI affordances that choose source/output paths from profile roots instead of accepting arbitrary text.

## Task Record

- Motivation:
  - Close a safety gap in media job creation before execution paths are wired.
- Design notes:
  - Used string boundary checks rather than filesystem canonicalization because planned media paths may not exist at API request time.
  - Loaded the profile for dry-run jobs as well as replacement jobs so validation is consistent across modes.
- Test coverage summary:
  - Added a failing app-service test proving outside source roots were accepted before implementation.
  - Added helper coverage for sibling-prefix rejection.
- Observability updates:
  - None. This is request validation before persistence.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: callers relying on cross-profile paths are rejected. Roll back by removing the path validation helper, service call, tests, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
